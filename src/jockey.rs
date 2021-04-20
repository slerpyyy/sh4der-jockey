use crate::pipeline::Pipeline;
use crate::util::*;
use gl::types::*;
use std::ffi::CString;
use std::io::Read;

pub struct Jockey {
    pub window: sdl2::video::Window,
    pub imgui: imgui::Context,
    pub imgui_sdl2: imgui_sdl2::ImguiSdl2,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub gl_context: sdl2::video::GLContext,
    pub event_pump: sdl2::EventPump,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub pipeline: Option<Pipeline>,
}

impl Jockey {
    pub fn init() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video = sdl_context.video().unwrap();

        {
            let gl_attr = video.gl_attr();
            gl_attr.set_context_profile(sdl2::video::GLProfile::Core);
            gl_attr.set_context_version(3, 0);
        }

        let title = format!(
            "Sh4derJockey (version {}-{})",
            env!("VERGEN_BUILD_SEMVER"),
            &env!("VERGEN_GIT_SHA")[0..7]
        );

        let window = video
            .window(&title, 1080, 720)
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

        Self {
            window,
            event_pump,
            imgui,
            imgui_sdl2,
            renderer,
            vao,
            vbo,
            gl_context,
            pipeline: None,
        }
    }

    pub fn update_pipeline<R: Read>(&mut self, reader: R) -> Option<()> {
        let object = serde_json::from_reader(reader).ok()?;
        let update = Pipeline::from_json(object)?;
        self.pipeline = Some(update);
        Some(())
    }

    pub fn draw(&self, width: f32, height: f32, time: f32) -> Option<()> {
        let pl = self.pipeline.as_ref()?;

        for stage in pl.stages.iter() {
            let (target_tex, target_fb) = stage
                .target
                .as_ref()
                .and_then(|s| pl.buffers.get(s).map(|(tex, fb, _)| (*tex, *fb)))
                .unwrap_or((0, 0));

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
                for (name, (tex_id, fb_id, k)) in pl.buffers.iter() {
                    let name = CString::new(name.as_bytes()).unwrap();
                    let loc = gl::GetUniformLocation(stage.prog_id, name.as_ptr());

                    gl::BindFramebuffer(gl::FRAMEBUFFER, *fb_id);
                    gl::BindTexture(gl::TEXTURE_2D, *tex_id);
                    gl::Uniform1i(loc, *k as _);
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
        }

        Some(())
    }
}
