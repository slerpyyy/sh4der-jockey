use std::io::Read;
use crate::pipeline::Pipeline;

pub struct Jockey {
    pub window: sdl2::video::Window,
    pub imgui: imgui::Context,
    pub imgui_sdl2: imgui_sdl2::ImguiSdl2,
    pub renderer: imgui_opengl_renderer::Renderer,
    pub gl_context: sdl2::video::GLContext,
    pub event_pump: sdl2::EventPump,
    pub vao: gl::types::GLuint,
    pub vbo: gl::types::GLuint,
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
}
