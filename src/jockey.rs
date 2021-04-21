use crate::pipeline::Pipeline;
use crate::util::*;
use gl::types::*;
use imgui::im_str;
use lazy_static::lazy_static;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use std::{ffi::CString, time::Instant};

lazy_static! {
    static ref JOCKEY_TITLE: String = {
        format!(
            "Sh4derJockey (version {}-{})",
            env!("VERGEN_BUILD_SEMVER"),
            &env!("VERGEN_GIT_SHA")[0..7]
        )
    };
}

/// A struct to keep the state of the tool.
///
/// This struct holds the render pipeline, as well as every type of context
/// required to keep the window alive. The main point of this struct is to
/// hide all the nasty details and keep the main function clean.
pub struct Jockey {
    pub window: sdl2::video::Window,
    pub imgui: imgui::Context,
    pub imgui_sdl2: imgui_sdl2::ImguiSdl2,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub gl_context: sdl2::video::GLContext,
    pub event_pump: sdl2::EventPump,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub pipeline: Pipeline,
    pub start_time: Instant,
    pub last_frame: Instant,
    pub frame_perf: RunningAverage<f32, 128>,
    pub done: bool,
}

impl std::fmt::Debug for Jockey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Jockey))
            .field("vao", &self.vao)
            .field("vbo", &self.vbo)
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

        gl::load_with(|s| video.gl_get_proc_address(s) as _);

        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        let imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui, &window);
        let renderer: imgui_opengl_renderer::Renderer =
            imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);
        let event_pump = sdl_context.event_pump().unwrap();

        let mut vao = 0;
        let mut vbo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
        }

        let pipeline = Pipeline::new();
        let frame_perf = RunningAverage::new();

        let start_time = Instant::now();
        let last_frame = start_time;

        let mut this = Self {
            window,
            event_pump,
            imgui,
            imgui_sdl2,
            renderer,
            vao,
            vbo,
            gl_context,
            pipeline,
            start_time,
            frame_perf,
            last_frame,
            done: false,
        };

        this.update_pipeline();
        this
    }

    /// Reload the render pipeline and replace the old one.
    ///
    /// This will load the `pipeline.json` from the specified file and
    /// attempt to read and compile all necessary shaders. If everything loaded
    /// successfully, the new Pipeline struct will stomp the old one.
    pub fn update_pipeline(&mut self) -> Option<()> {
        let reader = std::fs::File::open("pipeline.json").expect("could not open pipeline file");
        let object = serde_json::from_reader(reader).ok()?;
        let update = Pipeline::from_json(object)?;
        self.pipeline = update;
        Some(())
    }

    pub fn handle_events(&mut self) {
        let mut do_update_pipeline = false;

        for event in self.event_pump.poll_iter() {
            self.imgui_sdl2.handle_event(&mut self.imgui, &event);

            if self.imgui_sdl2.ignore_event(&event) {
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

                //Event::Window {
                //    win_event: WindowEvent::Resized(width, height),
                //    ..
                //} => {
                //    println!("resize detected {:?}", (width, height));
                //    self.window.set_size(width as _, height as _).unwrap();
                //}
                _ => {}
            }
        }

        // live shader reloading hype
        if do_update_pipeline {
            self.update_pipeline();
        }
    }

    /// Does all the OpenGL magic.
    ///
    /// This function iterates over all stages in the pipeline and renders
    /// them front to back. The only reason this function takes an `&mut self`
    /// is to record performance statistics.
    pub fn draw(&mut self) -> Option<()> {
        // compute uniforms
        let (width, height) = self.window.size();
        let time = self.start_time.elapsed().as_secs_f32();

        // render all shader stages
        for stage in self.pipeline.stages.iter_mut() {
            let stage_start = Instant::now();

            // get render target id
            let (target_tex, target_fb) = if let Some(name) = stage.target.as_ref() {
                let tex = &self.pipeline.buffers[name];
                (tex.id, tex.fb)
            } else {
                (0, 0) // The screen is always id=0
            };

            unsafe {
                // Use shader program
                gl::UseProgram(stage.prog_id);

                // Add uniforms
                {
                    let r_name = CString::new("R").unwrap();
                    let time_name = CString::new("time").unwrap();

                    let r_loc = gl::GetUniformLocation(stage.prog_id, r_name.as_ptr());
                    let time_loc = gl::GetUniformLocation(stage.prog_id, time_name.as_ptr());

                    gl::Uniform3f(r_loc, width as _, height as _, time);
                    gl::Uniform1f(time_loc, time);
                }

                // Add and bind uniform textures
                for (k, (name, tex)) in self.pipeline.buffers.iter().enumerate() {
                    let name = CString::new(name.as_bytes()).unwrap();
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());

                    gl::ActiveTexture(gl::TEXTURE0 + k as GLenum);
                    gl::BindTexture(gl::TEXTURE_2D, tex.id);
                    gl::BindFramebuffer(gl::FRAMEBUFFER, tex.fb);
                    gl::Uniform1i(loc, k as _);
                }

                // Specify render target
                gl::BindFramebuffer(gl::FRAMEBUFFER, target_fb);
                if target_fb != 0 {
                    gl::Viewport(0, 0, width as _, height as _);
                }

                // Specify fragment shader color output
                #[allow(temporary_cstring_as_ptr)]
                gl::BindFragDataLocation(
                    stage.prog_id,
                    0,
                    CString::new("out_color").unwrap().as_ptr(),
                );

                // Specify the layout of the vertex data
                #[allow(temporary_cstring_as_ptr)]
                let pos_attr = gl::GetAttribLocation(
                    stage.prog_id,
                    CString::new("position").unwrap().as_ptr(),
                );
                gl::EnableVertexAttribArray(pos_attr as GLuint);
                gl::VertexAttribPointer(
                    pos_attr as GLuint,
                    2,
                    gl::FLOAT,
                    gl::FALSE as GLboolean,
                    0,
                    std::ptr::null(),
                );

                // Draw stuff
                draw_fullscreen_rect(self.vao);

                // Generate mip maps
                gl::BindTexture(gl::TEXTURE_2D, target_tex);
                gl::GenerateMipmap(gl::TEXTURE_2D);
            }

            // log render time
            let stage_time = stage_start.elapsed().as_secs_f32();
            stage.perf.push(1000.0 * stage_time);
        }

        Some(())
    }

    /// Wrapper function for all the imgui stuff.
    pub fn build_ui(&mut self) {
        self.imgui_sdl2.prepare_frame(
            self.imgui.io_mut(),
            &self.window,
            &self.event_pump.mouse_state(),
        );

        // tell imgui what time it is
        let now = Instant::now();
        let delta_time = (now - self.last_frame).as_secs_f32();
        self.imgui.io_mut().delta_time = delta_time;
        self.last_frame = now;

        // record frame time
        self.frame_perf.push(1000.0 * delta_time);
        let frame_ms = self.frame_perf.get();

        // ui magic
        let ui = self.imgui.frame();
        ui.text(&*JOCKEY_TITLE);
        ui.separator();

        ui.text("...");
        ui.separator();

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

        self.imgui_sdl2.prepare_render(&ui, &self.window);
        self.renderer.render(ui);
    }
}
