use crate::pipeline::Pipeline;
use crate::util::*;
use gl::types::*;
use std::io::Read;
use std::{ffi::CString, time::Instant};

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
    pub fn title() -> String {
        format!(
            "Sh4derJockey (version {}-{})",
            env!("VERGEN_BUILD_SEMVER"),
            &env!("VERGEN_GIT_SHA")[0..7]
        )
    }

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
            .window(&title, 1080, 720)
            .position_centered()
            //.resizable()  // currently not working properly
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

        let start_time = Instant::now();

        Self {
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
        }
    }

    pub fn update_pipeline<R: Read>(&mut self, reader: R) -> Option<()> {
        let object = serde_json::from_reader(reader).ok()?;
        let update = Pipeline::from_json(object)?;
        self.pipeline = update;
        Some(())
    }

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
                (0, 0)
            };

            unsafe {
                // Use shader program
                gl::UseProgram(stage.prog_id);

                // Add uniforms
                {
                    let r_name = CString::new("R").unwrap();
                    let time_name = CString::new("time").unwrap();

                    let r_loc = gl::GetUniformLocation(stage.prog_id, r_name.as_ptr());
                    gl::Uniform3f(r_loc, width as _, height as _, time);

                    let time_loc = gl::GetUniformLocation(stage.prog_id, time_name.as_ptr());
                    gl::Uniform1f(time_loc, time);
                }

                // Add and bind uniform textures
                for (name, tex) in self.pipeline.buffers.iter() {
                    let name = CString::new(name.as_bytes()).unwrap();
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());

                    gl::BindFramebuffer(gl::FRAMEBUFFER, tex.fb);
                    gl::BindTexture(gl::TEXTURE_2D, tex.id);
                    gl::Uniform1i(loc, tex.slot as _);
                }

                // Specify render target
                gl::BindFramebuffer(gl::FRAMEBUFFER, target_fb);
                if target_fb != 0 {
                    gl::Viewport(0, 0, 1080, 720);
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
}
