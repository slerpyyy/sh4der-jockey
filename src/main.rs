extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate sdl2;

mod pipeline;
mod util;

use gl::types::*;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::str;
use std::time::Instant;
use util::*;

const VS_SRC: &'static str = "
#version 150
in vec2 position;
void main() {
    gl_Position = vec4(position, 0.0, 1.0);
}";

const FS_SRC: &'static str = "
#version 150
out vec4 out_color;
uniform vec3 R;
uniform float time;
void main() {
    out_color = vec4(gl_FragCoord.xy / R.xy, 0.5 + 0.5 * sin(R.z), 1.0);
}";

fn main() {
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

    let _gl_context = window
        .gl_create_context()
        .expect("Couldn't create GL context");

    gl::load_with(|s| video.gl_get_proc_address(s) as _);

    let mut imgui = imgui::Context::create();
    imgui.set_ini_filename(None);

    let mut imgui_sdl2 = imgui_sdl2::ImguiSdl2::new(&mut imgui, &window);
    let renderer =
        imgui_opengl_renderer::Renderer::new(&mut imgui, |s| video.gl_get_proc_address(s) as _);
    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut last_frame = Instant::now();
    let start_time = Instant::now();

    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    let mut vao = 0;
    let mut vbo = 0;

    unsafe {
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
    }

    'running: loop {
        use sdl2::event::Event;
        use sdl2::keyboard::Keycode;

        for event in event_pump.poll_iter() {
            imgui_sdl2.handle_event(&mut imgui, &event);
            if imgui_sdl2.ignore_event(&event) {
                continue;
            }

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        imgui_sdl2.prepare_frame(imgui.io_mut(), &window, &event_pump.mouse_state());

        let now = Instant::now();
        let delta = now - last_frame;
        let delta_s = delta.as_secs() as f32 + delta.subsec_nanos() as f32 / 1_000_000_000.0;
        last_frame = now;
        imgui.io_mut().delta_time = delta_s;

        let ui = imgui.frame();
        ui.show_demo_window(&mut true); // Zhe magic

        // compute uniforms
        let (width, height) = window.size();
        let time = start_time.elapsed().as_secs_f32();
        //println!("{:?}", (width, height, time));

        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (FULLSCREEN_RECT.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                mem::transmute(&FULLSCREEN_RECT[0]),
                gl::STATIC_DRAW,
            );

            // Use shader program
            gl::UseProgram(program);

            #[allow(temporary_cstring_as_ptr)]
            gl::BindFragDataLocation(program, 0, CString::new("out_color").unwrap().as_ptr());

            // Specify the layout of the vertex data
            #[allow(temporary_cstring_as_ptr)]
            let pos_attr =
                gl::GetAttribLocation(program, CString::new("position").unwrap().as_ptr());

            #[allow(temporary_cstring_as_ptr)]
            let r_loc = gl::GetUniformLocation(program, CString::new("R").unwrap().as_ptr());
            gl::Uniform3f(r_loc, width as _, height as _, time);

            #[allow(temporary_cstring_as_ptr)]
            let time_loc = gl::GetUniformLocation(program, CString::new("time").unwrap().as_ptr());
            gl::Uniform1f(time_loc, time);

            gl::EnableVertexAttribArray(pos_attr as GLuint);

            gl::VertexAttribPointer(
                pos_attr as GLuint,
                2,
                gl::FLOAT,
                gl::FALSE as GLboolean,
                0,
                ptr::null(),
            );

            // Draw a triangle from the 3 vertices
            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }

        imgui_sdl2.prepare_render(&ui, &window);
        renderer.render(ui);

        window.gl_swap_window();

        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
