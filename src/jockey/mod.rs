use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    ffi::CString,
    future::Future,
    hash::{Hash, Hasher},
    io::Write,
    mem::MaybeUninit,
    pin::Pin,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use gl::types::*;
use glutin::platform::run_return::EventLoopExtRunReturn;
use imgui::im_str;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use notify::Watcher;

use crate::util::*;

mod audio;
mod beatsync;
mod config;
mod midi;
mod network;
mod pipeline;
mod stage;
mod uniforms;

pub use audio::*;
pub use beatsync::*;
pub use config::*;
pub use midi::*;
pub use network::*;
pub use pipeline::*;
pub use stage::*;
pub use uniforms::*;

static mut PIPELINE_STALE: AtomicBool = AtomicBool::new(false);
static mut PROJECT_STALE: AtomicBool = AtomicBool::new(false);

/// A struct for all the ugly internals.
pub struct MegaContext {
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub watcher: Option<notify::RecommendedWatcher>,
    pub context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub ui_context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub events_loop: glutin::event_loop::EventLoop<()>,
    pub platform: WinitPlatform,
}

/// A struct to keep the state of the tool.
///
/// This struct holds the render pipeline, as well as every type of context
/// required to keep the window alive. The main point of this struct is to
/// hide all the nasty details and keep the main function clean.
pub struct Jockey {
    pub ctx: MegaContext,
    pub done: bool,
    pub frame_perf: RunningAverage<f32, 128>,
    pub beat_sync: BeatSync,
    pub last_build: Instant,
    pub last_frame: Instant,
    pub last_frame_ui: Instant,
    pub midi: Midi,
    pub audio: Audio,
    pub ndi: Ndi,
    pub pipeline_files: Vec<String>,
    pub pipeline_index: usize,
    pub pipeline: Pipeline,
    pub pipeline_partial: Option<Pin<PipelinePartial>>,
    pub time: f32,
    pub speed: f32,
    pub time_range: (f32, f32),
    pub frame: u32,
    pub alt_pressed: bool,
    pub console: String,
}

impl std::fmt::Debug for Jockey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Jockey))
            .field("vao", &self.ctx.vao)
            .field("vbo", &self.ctx.vbo)
            .field("pipeline", &self.pipeline)
            .finish()
    }
}

impl Jockey {
    /// Initializes the tool.
    ///
    /// This will spin up a Winit window, initialize Imgui,
    /// create a OpenGL context and more!
    pub fn init() -> Self {
        let config = Config::load_or_default();
        let audio = Audio::new(AUDIO_SAMPLES, &config);

        let events_loop = glutin::event_loop::EventLoop::new();
        let request = glutin::GlRequest::Latest;

        // Setup for imgui
        let ui_window_builder = glutin::window::WindowBuilder::new()
            .with_inner_size(glutin::dpi::LogicalSize::new(720.0, 640.0))
            .with_resizable(true)
            .with_title("Control Panel");

        #[cfg(target_os = "windows")]
        let ui_window_builder =
            glutin::platform::windows::WindowBuilderExtWindows::with_drag_and_drop(
                ui_window_builder,
                false,
            );

        let ui_context_builder = glutin::ContextBuilder::new().with_vsync(true);
        let ui_built_context = ui_context_builder
            .build_windowed(ui_window_builder, &events_loop)
            .expect("Failed to create windowed context");

        let ui_context = unsafe {
            ui_built_context
                .make_current()
                .expect("Failed to activate windowed context")
        };
        let ui_prog_addr = |s| ui_context.get_proc_address(s) as _;
        let mut imgui = imgui::Context::create();
        imgui.io_mut().config_flags |=
            imgui::ConfigFlags::DOCKING_ENABLE | imgui::ConfigFlags::VIEWPORTS_ENABLE;

        let mut ini_path = std::env::current_exe().unwrap();
        ini_path.set_file_name("imgui-layout.ini");
        imgui.set_ini_filename(Some(ini_path));

        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, ui_prog_addr);
        let mut platform = WinitPlatform::init(&mut imgui);
        let hidpi_factor = platform.hidpi_factor();
        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;
        platform.attach_window(imgui.io_mut(), ui_context.window(), HiDpiMode::Rounded);

        Self::init_imgui_style(imgui.style_mut());

        // Set up winit for OpenGL stuff
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_gl(request);

        let window_builder = glutin::window::WindowBuilder::new()
            .with_inner_size(glutin::dpi::LogicalSize::new(1280.0, 720.0))
            .with_resizable(true)
            .with_title("Sh4derJockey");

        #[cfg(target_os = "windows")]
        let window_builder = glutin::platform::windows::WindowBuilderExtWindows::with_drag_and_drop(
            window_builder,
            false,
        );

        let built_context = context_builder
            .build_windowed(window_builder, &events_loop)
            .expect("Failed to create windowed context");

        let context = unsafe {
            built_context
                .make_current()
                .expect("Failed to activate windowed context")
        };

        let prog_addr = |s| context.get_proc_address(s) as _;
        gl::load_with(prog_addr);

        // setup OpenGL
        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl_debug_check!();
        }

        let frame_perf = RunningAverage::new();

        let ctx = MegaContext {
            imgui,
            renderer,
            vao,
            vbo,
            watcher: None,
            context,
            ui_context,
            events_loop,
            platform,
        };

        let pipeline = Pipeline::splash_screen();
        let midi = Midi::new(&config);
        let ndi = Ndi::new();

        let console = "No pipeline has been built yet".into();

        let now = Instant::now();
        let mut this = Self {
            ctx,
            done: false,
            frame_perf,
            beat_sync: BeatSync::new(),
            last_build: now,
            last_frame: now,
            last_frame_ui: now,
            midi,
            audio,
            ndi,
            pipeline_files: Vec::new(),
            pipeline,
            pipeline_index: 0,
            pipeline_partial: None,
            time: 0.0,
            speed: 1.0,
            time_range: (0.0, 60.0),
            frame: 0,
            alt_pressed: false,
            console,
        };

        this.ctx.context = unsafe { this.ctx.context.make_current().unwrap() };
        this.update_pipeline();
        gl_debug_check!();
        this
    }

    // adapted from https://www.gitmemory.com/issue/ocornut/imgui/707/512669512
    #[rustfmt::skip]
    fn init_imgui_style(style: &mut imgui::Style) {
        fn gray(value: f32, alpha: f32) -> [f32; 4] {
            [value, value, value, alpha]
        }

        style.frame_rounding = 4.0;
        style.grab_rounding = 4.0;

        use imgui::StyleColor::*;
        style.colors[Text                   as usize] = gray(1.00, 1.00);
        style.colors[TextDisabled           as usize] = gray(0.40, 1.00);
        style.colors[WindowBg               as usize] = gray(0.16, 1.00);
        style.colors[ChildBg                as usize] = gray(0.16, 1.00);
        style.colors[PopupBg                as usize] = gray(0.08, 1.00);
        style.colors[Border                 as usize] = gray(0.08, 1.00);
        style.colors[BorderShadow           as usize] = gray(0.00, 1.00);
        style.colors[FrameBg                as usize] = gray(0.25, 1.00);
        style.colors[FrameBgHovered         as usize] = gray(0.20, 1.00);
        style.colors[FrameBgActive          as usize] = gray(0.12, 1.00);
        style.colors[TitleBg                as usize] = gray(0.12, 1.00);
        style.colors[TitleBgActive          as usize] = gray(0.08, 1.00);
        style.colors[TitleBgCollapsed       as usize] = gray(0.00, 0.50);
        style.colors[MenuBarBg              as usize] = gray(0.16, 1.00);
        style.colors[ScrollbarBg            as usize] = gray(0.16, 1.00);
        style.colors[ScrollbarGrab          as usize] = gray(0.25, 1.00);
        style.colors[ScrollbarGrabHovered   as usize] = gray(0.22, 1.00);
        style.colors[ScrollbarGrabActive    as usize] = gray(0.21, 1.00);
        style.colors[CheckMark              as usize] = gray(0.56, 1.00);
        style.colors[SliderGrab             as usize] = gray(0.46, 1.00);
        style.colors[SliderGrabActive       as usize] = gray(0.60, 1.00);
        style.colors[Button                 as usize] = gray(0.25, 1.00);
        style.colors[ButtonHovered          as usize] = gray(0.56, 1.00);
        style.colors[ButtonActive           as usize] = gray(0.53, 1.00);
        style.colors[Header                 as usize] = gray(0.25, 0.55);
        style.colors[HeaderHovered          as usize] = gray(0.60, 0.80);
        style.colors[HeaderActive           as usize] = gray(0.60, 1.00);
        style.colors[Separator              as usize] = gray(0.25, 1.00);
        style.colors[SeparatorHovered       as usize] = gray(0.40, 0.78);
        style.colors[SeparatorActive        as usize] = gray(0.40, 1.00);
        style.colors[ResizeGrip             as usize] = gray(0.60, 0.25);
        style.colors[ResizeGripHovered      as usize] = gray(0.60, 0.67);
        style.colors[ResizeGripActive       as usize] = gray(0.60, 0.95);
        style.colors[Tab                    as usize] = gray(0.16, 1.00);
        style.colors[TabHovered             as usize] = gray(0.60, 0.80);
        style.colors[TabActive              as usize] = gray(0.25, 1.00);
        style.colors[TabUnfocused           as usize] = gray(0.16, 1.00);
        style.colors[TabUnfocusedActive     as usize] = gray(0.16, 1.00);
        style.colors[PlotLines              as usize] = gray(1.00, 1.00);
        style.colors[PlotLinesHovered       as usize] = gray(1.00, 1.00);
        style.colors[PlotHistogram          as usize] = gray(1.00, 1.00);
        style.colors[PlotHistogramHovered   as usize] = gray(1.00, 1.00);
        style.colors[TextSelectedBg         as usize] = gray(0.60, 0.35);
        style.colors[DragDropTarget         as usize] = gray(1.00, 0.90);
        style.colors[NavHighlight           as usize] = gray(0.60, 1.00);
        style.colors[NavWindowingHighlight  as usize] = gray(1.00, 0.70);
        style.colors[NavWindowingDimBg      as usize] = gray(0.80, 0.20);
        style.colors[ModalWindowDimBg       as usize] = gray(0.80, 0.35);
    }

    /// Reload the render pipeline and replace the old one.
    ///
    /// This will load the `pipeline.yaml` from the specified file and
    /// attempt to read and compile all necessary shaders. If everything loaded
    /// successfully, the new Pipeline struct will stomp the old one.
    pub fn update_pipeline(&mut self) {
        // find pipeline files in working directory
        self.pipeline_files = std::fs::read_dir(".")
            .unwrap()
            .map(|s| s.unwrap().file_name().into_string().unwrap())
            .filter(|s| s.ends_with(".yaml"))
            .filter(|s| s != "config.yaml")
            .collect();

        log::info!("Found pipeline files: {:?}", &self.pipeline_files);

        // override pipeline index, if the user has no choice
        if self.pipeline_files.len() < 2 {
            self.pipeline_index = 0;
        }

        // get path of selected pipeline file
        let path = match self.pipeline_files.get(self.pipeline_index) {
            Some(s) => s,
            None => {
                log::warn!("Failed to find pipeline file");
                return;
            }
        };

        let screen_size = self.ctx.context.window().inner_size();
        let screen_size = (screen_size.width as u32, screen_size.height as u32);

        log::info!("Start building pipeline");
        self.pipeline_partial = Some(Box::pin(Pipeline::load(path.to_owned(), screen_size)));
    }

    fn update_pipeline_incremental(&mut self, timeout: Duration) {
        let start = Instant::now();
        while let Some(part) = self.pipeline_partial.as_mut() {
            if start.elapsed() > timeout {
                return;
            }

            if let Some(result) = futures::FutureExt::now_or_never(part) {
                self.pipeline_partial = None;

                // set waker on current working directory
                self.ctx.watcher = Some({
                    let event_fn = |_| unsafe { PIPELINE_STALE.store(true, Ordering::Release) };
                    let mut watcher = notify::immediate_watcher(event_fn).unwrap();
                    watcher
                        .watch(".", notify::RecursiveMode::Recursive)
                        .unwrap();

                    watcher
                });

                // unwrap pipeline build result
                let (new_pipeline, update) = match result {
                    Ok(t) => t,
                    Err(err) => {
                        self.console = format!("Failed to build pipeline:\n{}", err);
                        log::error!("{}", &self.console);
                        return;
                    }
                };

                // set new pipeline
                self.pipeline = new_pipeline;

                // log build time
                let build_time = self.last_build.elapsed().as_secs_f64();
                self.console = format!("Build pipeline over a span of {}s", build_time);
                log::info!("{}", &self.console);

                // toggle blend modes
                unsafe {
                    match self.pipeline.blending {
                        true => gl::Enable(gl::BLEND),
                        false => gl::Disable(gl::BLEND),
                    }
                    gl_debug_check!();
                }

                // copy audio configs
                self.audio.attack = update.smoothing_attack;
                self.audio.decay = update.smoothing_decay;
                if update.audio_samples != self.audio.size {
                    self.audio.resize(update.audio_samples);
                }

                // update ndi module
                let requests = self.pipeline.requested_ndi_sources.values();
                if let Err(err) = self.ndi.connect(&requests) {
                    log::error!("Failed to connect to NDI sources: {}", err);
                }
            }
        }
    }

    pub fn handle_events(&mut self) {
        take_mut::take(&mut self.ctx.context, |s| unsafe {
            s.make_current().unwrap()
        });

        let do_update_project = unsafe { PROJECT_STALE.swap(false, Ordering::AcqRel) };

        // reload all things that depend on the project-level config file
        if do_update_project {
            let config = Config::load_or_default();

            // the old midi struct must be dropped before the new one is created,
            // because it fails to connect to any common midi controller otherwise
            take_mut::take(&mut self.midi, |midi| {
                drop(midi);
                Midi::new(&config)
            });

            take_mut::take(&mut self.audio, |audio| {
                drop(audio);
                Audio::new(AUDIO_SAMPLES, &config)
            });
        }

        let platform = &mut self.ctx.platform;
        let events_loop = &mut self.ctx.events_loop;
        let imgui = &mut self.ctx.imgui;
        let window = self.ctx.context.window();
        let ui_window = self.ctx.ui_context.window();
        let pipeline = &mut self.pipeline;
        let alt_pressed = &mut self.alt_pressed;
        let mut done = false;

        self.midi.check_connections();
        self.midi.handle_input();

        let mut take_screenshot = false;
        let mut do_update_pipeline = unsafe { PIPELINE_STALE.swap(false, Ordering::AcqRel) }
            && self.last_build.elapsed().as_millis() > 300;

        let main_id = self.ctx.context.window().id();
        let ui_id = ui_window.id();

        events_loop.run_return(|e, _window_target, cf| {
            match e {
                glutin::event::Event::WindowEvent {
                    window_id,
                    ref event,
                } => {
                    if window_id == ui_id {
                        platform.handle_event(imgui.io_mut(), ui_window, &e);
                    }

                    match event {
                        glutin::event::WindowEvent::CloseRequested => done = true,

                        glutin::event::WindowEvent::Resized(size) if window_id == main_id => {
                            let width = size.width as u32;
                            let height = size.height as u32;
                            pipeline.resize_buffers(width, height);
                        }

                        #[allow(deprecated)]
                        glutin::event::WindowEvent::KeyboardInput { input, .. } => {
                            let shift = input.modifiers.shift();
                            let ctrl = input.modifiers.ctrl();
                            let alt = input.modifiers.alt();
                            let logo = input.modifiers.logo();
                            *alt_pressed = alt;

                            if Some(glutin::event::VirtualKeyCode::Return) == input.virtual_keycode
                                && input.state == glutin::event::ElementState::Pressed
                            {
                                if ctrl && !(shift || alt || logo) {
                                    do_update_pipeline = true;
                                }

                                // toggle fullscreen mode
                                if alt && !(shift || ctrl || logo) && window.id() == window_id {
                                    if window.fullscreen().is_some() {
                                        window.set_fullscreen(None);
                                    } else {
                                        let monitor =
                                            window.current_monitor().or(window.primary_monitor());

                                        let handle =
                                            Some(glutin::window::Fullscreen::Borderless(monitor));

                                        window.set_fullscreen(handle);
                                    }
                                }
                            }

                            if Some(glutin::event::VirtualKeyCode::S) == input.virtual_keycode
                                && input.state == glutin::event::ElementState::Pressed
                            {
                                if shift || ctrl {
                                    take_screenshot = true;
                                }
                            }
                        }

                        _ => (),
                    }
                }
                _ => (),
            };
            *cf = glutin::event_loop::ControlFlow::Exit;
        });

        self.done = done;

        if take_screenshot {
            self.save_frame();
        }

        // live shader reloading hype
        if do_update_pipeline {
            self.update_pipeline();
            self.last_build = Instant::now();
        }
    }

    /// Does all the OpenGL magic.
    ///
    /// This function iterates over all stages in the pipeline and renders
    /// them front to back. The only reason this function takes an `&mut self`
    /// is to record performance statistics.
    pub fn draw(&mut self) {
        take_mut::take(&mut self.ctx.context, |s| unsafe {
            s.make_current().unwrap()
        });

        // build pipeline a little
        self.update_pipeline_incremental(Duration::from_micros(50));

        // compute uniforms
        let screen_size = self.ctx.context.window().inner_size();
        let (width, height) = (screen_size.width as u32, screen_size.height as u32);
        let beat = self.beat_sync.beat();
        let now = Instant::now();
        let time = self.time;
        let delta = now.duration_since(self.last_frame).as_secs_f32();
        let frame = self.frame;
        self.time += delta * self.speed;
        self.last_frame = now;
        self.frame = self.frame.wrapping_add(1);

        {
            // update audio samples texture
            self.audio.update_samples();
            self.audio.update_fft();

            fn audio_tex_update(
                buffers: &mut HashMap<CString, Rc<dyn Texture>>,
                name: &CString,
                left: &[f32],
                right: &[f32],
            ) {
                if let Some(tex) = buffers.get_mut(name) {
                    unsafe {
                        alloca::with_slice(left.len() + right.len(), |buffer| {
                            let buffer = &mut *(buffer as *mut [MaybeUninit<f32>] as *mut _);

                            interlace(left, right, buffer);
                            Rc::get_mut(tex)
                                .unwrap()
                                .as_any_mut()
                                .downcast_mut::<Texture1D>()
                                .unwrap()
                                .write(buffer.as_ptr() as _);
                        })
                    }
                }
            }

            for (tex_name, src_name) in self.pipeline.requested_ndi_sources.iter() {
                let tex = self.pipeline.buffers.get_mut(tex_name).unwrap();
                let tex = Rc::get_mut(tex)
                    .unwrap()
                    .as_any_mut()
                    .downcast_mut::<Texture2D>()
                    .unwrap();
                self.ndi.update_texture(src_name, tex);
            }

            audio_tex_update(
                &mut self.pipeline.buffers,
                &SAMPLES_NAME,
                &self.audio.l_signal,
                &self.audio.r_signal,
            );
            audio_tex_update(
                &mut self.pipeline.buffers,
                &SPECTRUM_RAW_NAME,
                &self.audio.l_raw_spectrum,
                &self.audio.r_raw_spectrum,
            );
            audio_tex_update(
                &mut self.pipeline.buffers,
                &SPECTRUM_NAME,
                &self.audio.l_spectrum,
                &self.audio.r_spectrum,
            );
            audio_tex_update(
                &mut self.pipeline.buffers,
                &SPECTRUM_SMOOTH_NAME,
                &self.audio.l_spectrum_smooth,
                &self.audio.r_spectrum_smooth,
            );
            audio_tex_update(
                &mut self.pipeline.buffers,
                &SPECTRUM_SMOOTH_INTEGRATED_NAME,
                &self.audio.l_spectrum_smooth_integrated,
                &self.audio.r_spectrum_smooth_integrated,
            );
            audio_tex_update(
                &mut self.pipeline.buffers,
                &SPECTRUM_INTEGRATED_NAME,
                &self.audio.l_spectrum_integrated,
                &self.audio.r_spectrum_integrated,
            );
        }

        // render all shader stages
        for (pass_num, stage) in self.pipeline.stages.iter_mut().enumerate() {
            let stage_start = Instant::now();

            // skip stage if target is never used
            if !matches!(stage.kind, StageKind::Comp { .. }) {
                if let Some(name) = &stage.target {
                    if self.pipeline.buffers.get(name).is_none() {
                        continue;
                    }
                }
            }

            // get size of the render target
            let target_res = match stage.resolution() {
                Some(s) => s,
                _ => [width, height, 0],
            };

            unsafe {
                // Use shader program
                gl::UseProgram(stage.prog_id);
                gl_debug_check!();

                {
                    // Add time, beat, resolution and volume
                    let r_loc = gl::GetUniformLocation(stage.prog_id, R_NAME.as_ptr());
                    let k_loc = gl::GetUniformLocation(stage.prog_id, K_NAME.as_ptr());
                    let res_loc = gl::GetUniformLocation(stage.prog_id, RESOLUTION_NAME.as_ptr());
                    let pass_loc = gl::GetUniformLocation(stage.prog_id, PASS_INDEX_NAME.as_ptr());
                    let time_loc = gl::GetUniformLocation(stage.prog_id, TIME_NAME.as_ptr());
                    let frame_loc =
                        gl::GetUniformLocation(stage.prog_id, FRAME_COUNT_NAME.as_ptr());
                    let delta_loc = gl::GetUniformLocation(stage.prog_id, TIME_DELTA_NAME.as_ptr());
                    let beat_loc = gl::GetUniformLocation(stage.prog_id, BEAT_NAME.as_ptr());
                    let volume_loc = gl::GetUniformLocation(stage.prog_id, VOLUME_NAME.as_ptr());
                    let volume_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, VOLUME_INTEGRATED_NAME.as_ptr());
                    let bass_loc = gl::GetUniformLocation(stage.prog_id, BASS_NAME.as_ptr());
                    let mid_loc = gl::GetUniformLocation(stage.prog_id, MID_NAME.as_ptr());
                    let high_loc = gl::GetUniformLocation(stage.prog_id, HIGH_NAME.as_ptr());
                    let smooth_bass_loc =
                        gl::GetUniformLocation(stage.prog_id, BASS_SMOOTH_NAME.as_ptr());
                    let smooth_mid_loc =
                        gl::GetUniformLocation(stage.prog_id, MID_SMOOTH_NAME.as_ptr());
                    let smooth_high_loc =
                        gl::GetUniformLocation(stage.prog_id, HIGH_SMOOTH_NAME.as_ptr());

                    let bass_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, BASS_INTEGRATED_NAME.as_ptr());
                    let mid_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, MID_INTEGRATED_NAME.as_ptr());
                    let high_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, HIGH_INTEGRATED_NAME.as_ptr());
                    let smooth_bass_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, BASS_SMOOTH_INTEGRATED_NAME.as_ptr());
                    let smooth_mid_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, MID_SMOOTH_INTEGRATED_NAME.as_ptr());
                    let smooth_high_integrated_loc =
                        gl::GetUniformLocation(stage.prog_id, HIGH_SMOOTH_INTEGRATED_NAME.as_ptr());

                    gl::Uniform4f(
                        res_loc,
                        target_res[0] as f32,
                        target_res[1] as f32,
                        target_res[0] as f32 / target_res[1] as f32, // x/y
                        target_res[1] as f32 / target_res[0] as f32, // x/y
                    );
                    gl::Uniform3f(r_loc, target_res[0] as _, target_res[1] as _, time);
                    gl::Uniform3f(
                        volume_loc,
                        self.audio.volume[0], // average L/R
                        self.audio.volume[1], // L
                        self.audio.volume[2], // R
                    );
                    gl::Uniform3f(
                        bass_loc,
                        self.audio.bass[0],
                        self.audio.bass[1],
                        self.audio.bass[2],
                    );
                    gl::Uniform3f(
                        mid_loc,
                        self.audio.mid[0],
                        self.audio.mid[1],
                        self.audio.mid[2],
                    );
                    gl::Uniform3f(
                        high_loc,
                        self.audio.high[0],
                        self.audio.high[1],
                        self.audio.high[2],
                    );
                    gl::Uniform3f(
                        smooth_bass_loc,
                        self.audio.bass_smooth[0],
                        self.audio.bass_smooth[1],
                        self.audio.bass_smooth[2],
                    );
                    gl::Uniform3f(
                        smooth_mid_loc,
                        self.audio.mid_smooth[0],
                        self.audio.mid_smooth[1],
                        self.audio.mid_smooth[2],
                    );
                    gl::Uniform3f(
                        smooth_high_loc,
                        self.audio.high_smooth[0],
                        self.audio.high_smooth[1],
                        self.audio.high_smooth[2],
                    );
                    gl::Uniform3f(
                        volume_integrated_loc,
                        self.audio.volume_integrated[0], // average L/R
                        self.audio.volume_integrated[1], // L
                        self.audio.volume_integrated[2], // R
                    );
                    gl::Uniform3f(
                        bass_integrated_loc,
                        self.audio.bass_integrated[0],
                        self.audio.bass_integrated[1],
                        self.audio.bass_integrated[2],
                    );
                    gl::Uniform3f(
                        mid_integrated_loc,
                        self.audio.mid_integrated[0],
                        self.audio.mid_integrated[1],
                        self.audio.mid_integrated[2],
                    );
                    gl::Uniform3f(
                        high_integrated_loc,
                        self.audio.high_integrated[0],
                        self.audio.high_integrated[1],
                        self.audio.high_integrated[2],
                    );
                    gl::Uniform3f(
                        smooth_bass_integrated_loc,
                        self.audio.bass_smooth_integrated[0],
                        self.audio.bass_smooth_integrated[1],
                        self.audio.bass_smooth_integrated[2],
                    );
                    gl::Uniform3f(
                        smooth_mid_integrated_loc,
                        self.audio.mid_smooth_integrated[0],
                        self.audio.mid_smooth_integrated[1],
                        self.audio.mid_smooth_integrated[2],
                    );
                    gl::Uniform3f(
                        smooth_high_integrated_loc,
                        self.audio.high_smooth_integrated[0],
                        self.audio.high_smooth_integrated[1],
                        self.audio.high_smooth_integrated[2],
                    );
                    gl::Uniform2i(k_loc, pass_num as _, frame as _);
                    gl::Uniform1i(pass_loc, pass_num as _);
                    gl::Uniform1i(frame_loc, frame as _);
                    gl::Uniform1f(time_loc, time);
                    gl::Uniform1f(beat_loc, beat);
                    gl::Uniform1f(delta_loc, delta);
                    gl_debug_check!();
                }

                {
                    // Add sliders and buttons
                    let s_loc = gl::GetUniformLocation(stage.prog_id, SLIDERS_NAME.as_ptr());
                    let b_loc = gl::GetUniformLocation(stage.prog_id, BUTTONS_NAME.as_ptr());

                    let mut buttons = [0.0; 4 * MIDI_N];
                    for (k, button) in self.midi.buttons.iter().enumerate() {
                        buttons[k * 4 + 0] = button.0;
                        buttons[k * 4 + 1] = button.1.elapsed().as_secs_f32();
                        buttons[k * 4 + 2] = button.2.elapsed().as_secs_f32();
                        buttons[k * 4 + 3] = button.3 as f32;
                    }

                    gl::Uniform1fv(s_loc, self.midi.sliders.len() as _, &self.midi.sliders as _);
                    gl::Uniform4fv(b_loc, self.midi.buttons.len() as _, &buttons as _);
                    gl_debug_check!();
                }

                // Add custom uniforms
                for (name, uniform) in &stage.unis {
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());
                    uniform.bind(loc);
                    gl_debug_check!();
                }

                // Add vertex count uniform
                if let StageKind::Vert { count, .. } = stage.kind {
                    let loc = gl::GetUniformLocation(stage.prog_id, VERTEX_COUNT_NAME.as_ptr());
                    gl::Uniform1i(loc, count as _);
                    gl_debug_check!();
                }

                // Add and bind uniform texture dependencies
                for (k, name) in stage.deps.iter().enumerate() {
                    let tex = self.pipeline.buffers.get(name).unwrap();
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());
                    debug_assert_ne!(loc, -1);

                    gl::ActiveTexture(gl::TEXTURE0 + k as GLenum);
                    gl_debug_check!();

                    tex.bind(k as _);
                    gl_debug_check!();

                    gl::Uniform1i(loc, k as _);
                    gl_debug_check!();

                    let name_len = name.as_bytes().len();
                    let res_loc = alloca::with_bytes(name_len + 5, |buffer| {
                        let res_name = &mut *(buffer as *mut _ as *mut [u8]);

                        res_name[..name_len].copy_from_slice(name.as_bytes());
                        res_name[name_len..].copy_from_slice("_res\0".as_bytes());

                        gl::GetUniformLocation(stage.prog_id, res_name.as_ptr() as _)
                    });

                    let res = tex.resolution();
                    gl_debug_check!();

                    gl::Uniform4f(
                        res_loc,
                        res[0] as _,
                        res[1] as _,
                        res[2] as _,
                        res[0] as f32 / res[1] as f32,
                    );
                    gl_debug_check!();
                }
            }

            match &stage.kind {
                StageKind::Comp { dispatch, .. } => unsafe {
                    gl::DispatchCompute(dispatch[0], dispatch[1], dispatch[2]);
                    gl::MemoryBarrier(
                        gl::TEXTURE_UPDATE_BARRIER_BIT
                            | gl::TEXTURE_FETCH_BARRIER_BIT
                            | gl::SHADER_IMAGE_ACCESS_BARRIER_BIT,
                    );
                    gl_debug_check!();
                },
                _ => unsafe {
                    debug_assert_eq!(target_res[2], 0);

                    // get render target id
                    let (target_tex, target_fb) = if let Some(name) = &stage.target {
                        let tex = self.pipeline.buffers.get(name).unwrap();
                        let tex_id = tex.texture_id();
                        let fb_id = tex
                            .framebuffer_id()
                            .expect("Render target should be a framebuffer");
                        (tex_id, fb_id)
                    } else {
                        (0, 0) // The screen is always id=0
                    };

                    // Specify render target
                    gl::BindFramebuffer(gl::FRAMEBUFFER, target_fb);
                    gl::Viewport(0, 0, target_res[0] as _, target_res[1] as _);
                    gl_debug_check!();

                    // Specify fragment shader color output
                    gl::BindFragDataLocation(stage.prog_id, 0, OUT_COLOR_NAME.as_ptr());
                    gl_debug_check!();

                    // Specify the layout of the vertex data
                    let pos_attr = gl::GetAttribLocation(stage.prog_id, POSITION_NAME.as_ptr());
                    if pos_attr != -1 {
                        gl_debug_check!();
                        gl::EnableVertexAttribArray(pos_attr as GLuint);
                        gl_debug_check!();
                        gl::VertexAttribPointer(
                            pos_attr as GLuint,
                            2,
                            gl::FLOAT,
                            gl::FALSE as GLboolean,
                            0,
                            std::ptr::null(),
                        );
                    }
                    gl_debug_check!();

                    // Set blend mode
                    if self.pipeline.blending {
                        let (src, dst) = stage.blend.unwrap_or((gl::ONE, gl::ZERO));
                        gl::BlendFunc(src, dst);
                        gl_debug_check!();
                    }

                    // Draw stuff
                    if let StageKind::Vert {
                        count,
                        mode,
                        thickness,
                        ..
                    } = stage.kind
                    {
                        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
                        gl::Clear(gl::COLOR_BUFFER_BIT);
                        gl_debug_check!();

                        gl::PointSize(thickness);
                        gl::LineWidth(thickness);
                        gl_debug_check!();

                        draw_vertices(self.ctx.vao, count, mode);
                        gl_debug_check!();
                    } else {
                        draw_fullscreen(self.ctx.vao);
                        gl_debug_check!();
                    }

                    // Generate mip maps
                    // don't do it for the screen buffer
                    if target_tex != 0 {
                        gl::BindTexture(gl::TEXTURE_2D, target_tex);
                        gl::GenerateMipmap(gl::TEXTURE_2D);
                        gl_debug_check!();
                    }

                    // swap buffers
                    if let Some(name) = &stage.target {
                        self.pipeline.buffers.get(name).unwrap().swap();
                    }
                },
            }

            // log render time
            let stage_time = stage_start.elapsed().as_secs_f32();
            stage.perf.push(1000.0 * stage_time);
        }

        self.ctx.context.swap_buffers().unwrap();
    }

    /// Wrapper function for all the imgui stuff.
    pub fn update_ui(&mut self) {
        take_mut::take(&mut self.ctx.ui_context, |s| unsafe {
            s.make_current().unwrap()
        });

        let io = self.ctx.imgui.io_mut();
        self.ctx
            .platform
            .prepare_frame(io, self.ctx.ui_context.window())
            .expect("Failed to start frame");

        // tell imgui what time it is
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_ui).as_secs_f32();
        io.delta_time = delta_time;
        self.last_frame_ui = now;

        // record frame time
        self.frame_perf.push(1000.0 * delta_time);
        let frame_ms = self.frame_perf.get();

        // title section
        let ui = self.ctx.imgui.frame();

        // create docking space
        unsafe {
            let flags = imgui::sys::ImGuiDockNodeFlags_None as i32;
            let viewport = imgui::sys::igGetMainViewport();
            let window_class = imgui::sys::ImGuiWindowClass_ImGuiWindowClass();
            imgui::sys::igDockSpaceOverViewport(viewport, flags, window_class);
        }

        if let Some(window) = imgui::Window::new(im_str!("Pipelines")).begin(&ui) {
            if ui.button_with_size(im_str!("Select project folder"), [0.0; 2]) {
                std::thread::spawn(|| {
                    let choice = nfd::open_pick_folder(None);
                    let path = match choice {
                        Ok(nfd::Response::Okay(s)) => s,
                        _ => return,
                    };

                    log::info!("Setting cwd to {}", &path);
                    if let Err(err) = std::env::set_current_dir(path) {
                        log::error!("Failed setting cwd: {}", err);
                    }

                    unsafe {
                        PIPELINE_STALE.store(true, Ordering::Release);
                        PROJECT_STALE.store(true, Ordering::Release);
                    }
                });
            }

            ui.separator();
            match self.pipeline_files.len() {
                0 => ui.text("No yaml file found"),
                1 => ui.text("Only one yaml file found"),
                _ => {
                    for (k, file) in self.pipeline_files.iter().enumerate() {
                        let cst = CString::new(file.as_bytes()).unwrap();
                        let ims = unsafe { imgui::ImStr::from_cstr_unchecked(&cst) };
                        if ui.button_with_size(ims, [256.0, 18.0]) {
                            self.pipeline_index = k;
                            unsafe { PIPELINE_STALE.store(true, Ordering::Release) }
                        }
                    }
                }
            }

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Timeline")).begin(&ui) {
            if ui.button_with_size(im_str!("Play"), [64.0, 18.0]) {
                self.speed = 1.0;
            }

            ui.same_line();
            if ui.button_with_size(im_str!("Stop"), [64.0, 18.0]) {
                self.speed = 0.0;
            }

            ui.same_line();
            if ui.button_with_size(im_str!("Reset"), [64.0, 18.0]) {
                self.time = 0.0;
                self.frame = 0;
            }

            let (start, end) = &mut self.time_range;
            imgui::Slider::new(im_str!("time"))
                .range(*start..=*end)
                .build(&ui, &mut self.time);
            imgui::Slider::new(im_str!("speed"))
                .range(-2.0..=2.0)
                .build(&ui, &mut self.speed);

            ui.set_next_item_width(64.0);
            ui.input_float(im_str!("start"), start).build();

            ui.same_line();
            ui.set_next_item_width(64.0);
            ui.input_float(im_str!("end"), end).build();

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Buttons")).begin(&ui) {
            for k in 0..self.midi.buttons.len() {
                let token = ui.push_id(i32::MAX - k as i32);
                if !self.alt_pressed {
                    if ui.small_button(im_str!("bind")) {
                        self.midi.bind_button(k);
                    }
                } else {
                    if ui.small_button(im_str!("unbind")) {
                        self.midi.unbind_button(k);
                    }
                }
                token.pop();
                ui.same_line();

                let mut buffer = [0_u8; 16];
                write!(buffer.as_mut(), "button{}\0", k).unwrap();
                let cstr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(&buffer) };
                let ims = unsafe { imgui::ImStr::from_cstr_unchecked(cstr) };
                let button = ui.button_with_size(ims, [64.0, 18.0]);

                // button is false while it's held down.
                // we consider button to be pressed when the mouse is over button
                // and the mouse is held down
                if self.midi.buttons[k].0 == 0.0
                    && ui.is_mouse_down(imgui::MouseButton::Left)
                    && ui.is_item_hovered()
                {
                    self.midi.buttons[k].0 = 1.0;
                    self.midi.buttons[k].1 = Instant::now();
                    self.midi.buttons[k].3 += 1;
                }

                // button is true when it gets released
                if self.midi.buttons[k].0 != 0.0 && button {
                    self.midi.buttons[k].0 = 0.0;
                    self.midi.buttons[k].2 = Instant::now();
                }

                if k & 3 != 3 {
                    ui.same_line();
                }
            }

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Sliders")).begin(&ui) {
            for k in 0..self.midi.sliders.len() {
                let token = ui.push_id(k as i32);
                if !self.alt_pressed {
                    if ui.small_button(im_str!("bind")) {
                        self.midi.bind_slider(k);
                    }
                } else {
                    if ui.small_button(im_str!("unbind")) {
                        self.midi.unbind_slider(k);
                    }
                }
                token.pop();
                ui.same_line();

                let mut buffer = [0_u8; 16];
                write!(buffer.as_mut(), "slider{}\0", k).unwrap();
                let cstr = unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(&buffer) };
                let ims = unsafe { imgui::ImStr::from_cstr_unchecked(cstr) };
                let slider = &mut self.midi.sliders[k];
                imgui::Slider::new(ims).range(0.0..=1.0).build(&ui, slider);
            }

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Audio")).begin(&ui) {
            ui.plot_lines(im_str!("left"), &self.audio.l_signal).build();
            ui.plot_lines(im_str!("right"), &self.audio.r_signal)
                .build();

            ui.separator();
            ui.plot_lines(im_str!("left FFT"), self.audio.l_raw_spectrum.as_slice())
                .build();
            ui.plot_lines(im_str!("right FFT"), self.audio.r_raw_spectrum.as_slice())
                .build();

            ui.separator();
            ui.plot_lines(im_str!("nice L FFT"), self.audio.l_spectrum.as_slice())
                .build();
            ui.plot_lines(im_str!("nice R FFT"), self.audio.r_spectrum.as_slice())
                .build();

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Beat Sync")).begin(&ui) {
            if ui.button_with_size(im_str!("Tab here"), [128.0, 32.0]) {
                self.beat_sync.trigger();
            }
            ui.same_line();
            ui.text(format!(
                "BPM: {}\ncount: {}",
                self.beat_sync.bpm(),
                self.beat_sync.count
            ));

            imgui::ProgressBar::new(self.beat_sync.beat().fract()).build(&ui);

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Performance")).begin(&ui) {
            ui.text(format!(
                "FPS: {:.2} ({:.2} ms)",
                1000.0 / frame_ms,
                frame_ms
            ));

            ui.plot_lines(im_str!("dt [ms]"), &self.frame_perf.buffer)
                .build();

            let mut stage_sum_ms = 0.0;
            for (k, stage) in self.pipeline.stages.iter().enumerate() {
                let stage_ms = stage.perf.get();
                stage_sum_ms += stage_ms;
                if let Some(tex_name) = stage.target.as_ref() {
                    ui.text(format!(
                        "Stage {}: {:.4} ms (-> {:?})",
                        k, stage_ms, tex_name
                    ));
                } else {
                    ui.text(format!("Stage {}: {:.4} ms", k, stage_ms));
                }
            }

            ui.text(format!(
                "Total: {:.4} ms ({:.2}% stress)",
                stage_sum_ms,
                100.0 * stage_sum_ms / frame_ms
            ));

            window.end();
        }

        if let Some(window) = imgui::Window::new(im_str!("Build Output")).begin(&ui) {
            ui.text(&self.console);
            window.end();
        }

        // update ui
        self.ctx
            .platform
            .prepare_render(&ui, self.ctx.ui_context.window());

        // render and swap buffers
        self.ctx.renderer.render(ui);
        self.ctx.ui_context.swap_buffers().unwrap();
    }

    pub fn save_frame(&mut self) {
        take_mut::take(&mut self.ctx.context, |s| unsafe {
            s.make_current().unwrap()
        });

        let screen_size = self.ctx.context.window().inner_size();
        let (width, height) = (screen_size.width as u32, screen_size.height as u32);

        let mut img = image::ImageBuffer::<image::Rgb<u8>, Vec<u8>>::new(width, height);
        let data = img.as_flat_samples_mut().as_mut_slice().as_mut_ptr();

        unsafe {
            gl::ReadnPixels(
                0,
                0,
                width as _,
                height as _,
                gl::RGB,
                gl::UNSIGNED_BYTE,
                (3 * width * height) as _,
                data as _,
            );
        }

        image::imageops::flip_vertical_in_place(&mut img);

        let mut hasher = DefaultHasher::new();
        Instant::now().hash(&mut hasher);
        img.hash(&mut hasher);
        let hash = hasher.finish();

        let file_name = format!("frame-{}.png", hash);
        img.save(file_name).unwrap();
    }
}
