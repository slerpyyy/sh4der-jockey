use crate::util::*;
use gl::types::*;
use imgui::im_str;
use lazy_static::lazy_static;
use sdl2::{
    event::{Event, WindowEvent},
    keyboard::{Keycode, Mod},
};
use std::{
    ffi::CString,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

mod audio;
mod midi;
mod pipeline;
mod stage;

pub use audio::*;
pub use midi::*;
pub use pipeline::*;
pub use stage::*;

lazy_static! {
    static ref JOCKEY_TITLE: String = {
        format!(
            "Sh4derJockey (version {}-{})",
            env!("VERGEN_BUILD_SEMVER"),
            &env!("VERGEN_GIT_SHA")[0..7]
        )
    };
}

static mut FILE_CHANGE: AtomicBool = AtomicBool::new(false);

/// A struct for all the ugly internals.
pub struct MegaContext {
    pub event_pump: sdl2::EventPump,
    pub gl_context: sdl2::video::GLContext,
    pub imgui_sdl2: imgui_sdl2::ImguiSdl2,
    pub imgui: imgui::Context,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub watcher: notify::RecommendedWatcher,
    pub window: sdl2::video::Window,
}

/// A struct to keep the state of the tool.
///
/// This struct holds the render pipeline, as well as every type of context
/// required to keep the window alive. The main point of this struct is to
/// hide all the nasty details and keep the main function clean.
pub struct Jockey {
    pub beat_delta: RunningAverage<f32, 8>,
    pub ctx: MegaContext,
    pub done: bool,
    pub frame_perf: RunningAverage<f32, 128>,
    pub last_beat: Instant,
    pub last_build: Instant,
    pub last_frame: Instant,
    pub midi: Midi<8>,
    pub audio: Audio,
    pub pipeline: Pipeline,
    pub start_time: Instant,
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
    /// Returns a string containing the name of the program, the current
    /// version and commit hash.
    pub fn title() -> String {
        JOCKEY_TITLE.clone()
    }

    /// Initializes the tool.
    ///
    /// This will spin up a SDL2 window, initialize Imgui,
    /// create a OpenGL context and more!
    pub fn init() -> Self {
        // We need to init audio before SDL
        // I have no clue why
        // https://github.com/RustAudio/cpal/pull/330
        // this discusses "init A first or B first" so they are related somehow.
        let audio = Audio::new();

        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();

        {
            let gl_attr = video.gl_attr();
            gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
            gl_attr.set_context_version(3, 0);
        }

        let title = Self::title();
        let window = video
            .window(&title, 1280, 720)
            .position_centered()
            .resizable()
            .opengl()
            .allow_highdpi()
            .build()
            .unwrap();

        let gl_context = window
            .gl_create_context()
            .expect("Couldn't create GL context");

        let prog_addr = |s| video.gl_get_proc_address(s) as _;
        gl::load_with(prog_addr);

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui, &window);
        let renderer = imgui_opengl_renderer::Renderer::new(&mut imgui, prog_addr);
        let event_pump = sdl_context.event_pump().unwrap();

        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
        }

        Self::init_imgui_style(imgui.style_mut());

        let pipeline = Pipeline::new();
        let last_build = Instant::now();
        let frame_perf = RunningAverage::new();

        #[rustfmt::skip]
        let mut watcher = notify::immediate_watcher(
            |_| unsafe { FILE_CHANGE.store(true, Ordering::Relaxed) }
        ).unwrap();

        notify::Watcher::watch(&mut watcher, ".", notify::RecursiveMode::Recursive).unwrap();

        let midi = Midi::new();

        let ctx = MegaContext {
            event_pump,
            gl_context,
            imgui_sdl2,
            imgui,
            renderer,
            vao,
            vbo,
            watcher,
            window,
        };

        let mut beat_delta = RunningAverage::new();
        beat_delta.buffer.fill(1.0);

        let start_time = Instant::now();
        let last_frame = start_time;
        let last_beat = start_time;

        let mut this = Self {
            beat_delta,
            ctx,
            done: false,
            frame_perf,
            last_beat,
            last_build,
            last_frame,
            midi,
            audio,
            pipeline,
            start_time,
        };

        gl_debug_check!();

        this.update_pipeline();
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
        let start_time = Instant::now();
        let update = match Pipeline::load(&self.ctx.window) {
            Ok(pl) => pl,
            Err(err) => {
                eprintln!("Failed to load pipeline:\n{}", err);
                return;
            }
        };

        self.pipeline = update;
        println!("\n{:?}\n", self.pipeline);

        let time = start_time.elapsed().as_secs_f64();
        println!("Build pipeline in {}ms", 1000.0 * time);
    }

    pub fn handle_events(&mut self) {
        self.midi.check_connections();
        self.midi.handle_input();

        self.audio.update_samples();

        let mut do_update_pipeline = unsafe { FILE_CHANGE.swap(false, Ordering::Relaxed) }
            && self.last_build.elapsed().as_millis() > 100;

        for event in self.ctx.event_pump.poll_iter() {
            self.ctx
                .imgui_sdl2
                .handle_event(&mut self.ctx.imgui, &event);

            if self.ctx.imgui_sdl2.ignore_event(&event) {
                continue;
            }

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => self.done = true,

                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod,
                    ..
                } if keymod & Mod::LCTRLMOD != Mod::NOMOD => do_update_pipeline = true,

                Event::Window {
                    win_event: WindowEvent::Resized(width, height),
                    ..
                } if !do_update_pipeline => {
                    self.pipeline.resize_buffers(width as _, height as _);
                }
                _ => {}
            }
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
        lazy_static! {
            static ref R_NAME: CString = CString::new("R").unwrap();
            static ref K_NAME: CString = CString::new("K").unwrap();
            static ref RESOLUTION_NAME: CString = CString::new("resolution").unwrap();
            static ref PASS_INDEX_NAME: CString = CString::new("passIndex").unwrap();
            static ref TIME_NAME: CString = CString::new("time").unwrap();
            static ref BEAT_NAME: CString = CString::new("beat").unwrap();
            static ref SLIDERS_NAME: CString = CString::new("sliders").unwrap();
            static ref BUTTONS_NAME: CString = CString::new("buttons").unwrap();
            static ref VERTEX_COUNT_NAME: CString = CString::new("vertexCount").unwrap();
            static ref OUT_COLOR_NAME: CString = CString::new("out_color").unwrap();
            static ref POSITION_NAME: CString = CString::new("position").unwrap();
            static ref SAMPLES_NAME: CString = CString::new("samples").unwrap();
        }

        // compute uniforms
        let (width, height) = self.ctx.window.size();
        let time = self.start_time.elapsed().as_secs_f32();
        let beat = self.last_beat.elapsed().as_secs_f32() / self.beat_delta.get();
        gl_debug_check!();

        {
            // update audio samples texture
            let sample_name: &CString = &SAMPLES_NAME;
            let samples_tex = self.pipeline.buffers.get_mut(sample_name).unwrap();
            let interlaced_samples = interlace(&self.audio.l_signal, &self.audio.r_signal);

            if let Some(tex) = samples_tex.as_any_mut().downcast_mut::<TextureStruct>() {
                tex.write(&interlaced_samples);
            }

            gl_debug_check!();
        }

        // render all shader stages
        for (pass_num, stage) in self.pipeline.stages.iter_mut().enumerate() {
            let stage_start = Instant::now();

            // get size of the render target
            let target_res = match stage.resolution() {
                Some([w, h, 0]) => (w, h),
                _ => (width, height),
            };

            unsafe {
                // Use shader program
                gl::UseProgram(stage.prog_id);
                gl_debug_check!();

                {
                    // Add time, beat and resolution
                    let r_loc = gl::GetUniformLocation(stage.prog_id, R_NAME.as_ptr());
                    let k_loc = gl::GetUniformLocation(stage.prog_id, K_NAME.as_ptr());
                    let res_loc = gl::GetUniformLocation(stage.prog_id, RESOLUTION_NAME.as_ptr());
                    let pass_loc = gl::GetUniformLocation(stage.prog_id, PASS_INDEX_NAME.as_ptr());
                    let time_loc = gl::GetUniformLocation(stage.prog_id, TIME_NAME.as_ptr());
                    let beat_loc = gl::GetUniformLocation(stage.prog_id, BEAT_NAME.as_ptr());

                    gl::Uniform4f(
                        res_loc,
                        target_res.0 as f32,
                        target_res.1 as f32,
                        target_res.0 as f32 / target_res.1 as f32,
                        target_res.1 as f32 / target_res.0 as f32,
                    );
                    gl::Uniform3f(r_loc, target_res.0 as _, target_res.1 as _, time);
                    gl::Uniform1i(k_loc, pass_num as _);
                    gl::Uniform1i(pass_loc, pass_num as _);
                    gl::Uniform1f(time_loc, time);
                    gl::Uniform1f(beat_loc, beat);
                    gl_debug_check!();
                }

                {
                    // Add sliders and buttons
                    let s_loc = gl::GetUniformLocation(stage.prog_id, SLIDERS_NAME.as_ptr());
                    let b_loc = gl::GetUniformLocation(stage.prog_id, BUTTONS_NAME.as_ptr());

                    let mut buttons = [0.0; 8];
                    for (k, last_press) in self.midi.buttons.iter().enumerate() {
                        buttons[k] = last_press.elapsed().as_secs_f32();
                    }

                    gl::Uniform1fv(s_loc, self.midi.sliders.len() as _, &self.midi.sliders as _);
                    gl::Uniform1fv(b_loc, buttons.len() as _, &buttons as _);
                    gl_debug_check!();
                }

                // Add vertex count uniform
                if let StageKind::Vert { count, .. } = stage.kind {
                    let loc = gl::GetUniformLocation(stage.prog_id, VERTEX_COUNT_NAME.as_ptr());
                    gl::Uniform1f(loc, count as _);
                    gl_debug_check!();
                }

                // Add and bind uniform texture dependencies
                for (k, name) in stage.deps.iter().enumerate() {
                    let tex = self.pipeline.buffers.get(name).unwrap();
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());

                    gl::ActiveTexture(gl::TEXTURE0 + k as GLenum);
                    tex.activate();

                    gl::Uniform1i(loc, k as _);
                    gl_debug_check!();
                }
            }

            match &stage.kind {
                StageKind::Comp { tex_dim, .. } => unsafe {
                    gl::DispatchCompute(tex_dim[0], tex_dim[1].max(1), tex_dim[2].max(1));
                    gl::MemoryBarrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);
                    gl_debug_check!();
                },
                _ => {
                    // get render target id
                    let (target_tex, target_fb) = if let Some(name) = &stage.target {
                        let tex = &self.pipeline.buffers[name];
                        let tex = tex.as_any().downcast_ref::<TextureStruct>().unwrap();

                        if let TextureKind::FrameBuffer { fb, .. } = tex.kind {
                            (tex.id, fb)
                        } else {
                            panic!("No framebuffer for render target!")
                        }
                    } else {
                        (0, 0) // The screen is always id=0
                    };

                    unsafe {
                        // Specify render target
                        gl::BindFramebuffer(gl::FRAMEBUFFER, target_fb);
                        gl::Viewport(0, 0, target_res.0 as _, target_res.1 as _);
                        gl_debug_check!();

                        // Specify fragment shader color output
                        gl::BindFragDataLocation(stage.prog_id, 0, OUT_COLOR_NAME.as_ptr());
                        gl_debug_check!();

                        // Specify the layout of the vertex data
                        let pos_attr = gl::GetAttribLocation(stage.prog_id, POSITION_NAME.as_ptr());
                        if pos_attr != -1 {
                            gl::EnableVertexAttribArray(pos_attr as GLuint);
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

                        // Draw stuff
                        if let StageKind::Vert { count, mode, .. } = stage.kind {
                            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
                            gl::Clear(gl::COLOR_BUFFER_BIT);

                            draw_anything(self.ctx.vao, count, mode)
                        } else {
                            draw_fullscreen_rect(self.ctx.vao);
                        }
                        gl_debug_check!();

                        // Generate mip maps
                        // don't do it for the screen buffer
                        if target_tex != 0 {
                            gl::BindTexture(gl::TEXTURE_2D, target_tex);
                            gl::GenerateMipmap(gl::TEXTURE_2D);
                        }
                        gl_debug_check!();
                    }
                }
            }

            // log render time
            let stage_time = stage_start.elapsed().as_secs_f32();
            stage.perf.push(1000.0 * stage_time);
        }
    }

    /// Wrapper function for all the imgui stuff.
    pub fn build_ui(&mut self) {
        self.ctx.imgui_sdl2.prepare_frame(
            self.ctx.imgui.io_mut(),
            &self.ctx.window,
            &self.ctx.event_pump.mouse_state(),
        );

        // tell imgui what time it is
        let now = Instant::now();
        let delta_time = (now - self.last_frame).as_secs_f32();
        self.ctx.imgui.io_mut().delta_time = delta_time;
        self.last_frame = now;

        // record frame time
        self.frame_perf.push(1000.0 * delta_time);
        let frame_ms = self.frame_perf.get();

        // title section
        let ui = self.ctx.imgui.frame();
        ui.text(&*JOCKEY_TITLE);
        ui.separator();

        // sliders
        for k in 0..self.midi.sliders.len() {
            let token = ui.push_id(k as i32);
            if ui.small_button(im_str!("bind")) {
                self.midi.auto_bind_slider(k);
            }
            token.pop(&ui);
            ui.same_line(0.0);
            let name = format!("slider{}", k);
            let cst = std::ffi::CString::new(name).unwrap();
            let ims = unsafe { imgui::ImStr::from_cstr_unchecked(&cst) };
            let slider = &mut self.midi.sliders[k];
            imgui::Slider::new(ims).range(0.0..=1.0).build(&ui, slider);
        }

        // buttons
        for k in 0..self.midi.buttons.len() {
            let token = ui.push_id(-(k as i32) - 1);
            if ui.small_button(im_str!("bind")) {
                self.midi.auto_bind_button(k);
            }
            token.pop(&ui);
            ui.same_line(0.0);
            let name = format!("button{}", k);
            let cst = std::ffi::CString::new(name).unwrap();
            let ims = unsafe { imgui::ImStr::from_cstr_unchecked(&cst) };
            if ui.button(ims, [64.0, 18.0]) {
                self.midi.buttons[k] = Instant::now();
            }
            if k & 3 != 3 {
                ui.same_line(0.0)
            }
        }

        ui.separator();

        ui.plot_lines(im_str!("left"), &self.audio.l_signal).build();
        ui.plot_lines(im_str!("right"), &self.audio.r_signal)
            .build();

        ui.separator();

        // beat sync
        if ui.button(im_str!("Tab here"), [128.0, 32.0]) {
            let delta = self.last_beat.elapsed().as_secs_f32();
            self.beat_delta.push(delta);
            self.last_beat = Instant::now();
        }
        ui.same_line(0.0);
        ui.text(format! {
            "BPM: {}\nCycle: {}", 60.0 / self.beat_delta.get(), self.beat_delta.index
        });

        ui.separator();

        // perf monitor
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

        // update ui
        self.ctx.imgui_sdl2.prepare_render(&ui, &self.ctx.window);
        self.ctx.renderer.render(ui);
    }
}
