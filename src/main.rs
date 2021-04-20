extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate sdl2;

mod jockey;
mod pipeline;
mod util;

use getopts::Options;
use gl::types::*;
use jockey::*;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::ffi::CString;
use std::fs::File;
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

fn print_usage(name: &str, opts: &Options) {
    println!("Usage: {} [option]\n", name);
    print!("A custom VJ tool written by sp4ghet and slerpy.");
    println!("{}", opts.usage(""));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bin_name = args.get(0).unwrap();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message");
    opts.optflag("i", "init", "set up a simple example project");

    let matches = opts.parse(&args[1..]).expect("failed to parse args");
    if matches.opt_present("h") {
        print_usage(bin_name, &opts);
        return;
    }

    if matches.opt_present("i") {
        todo!()
        //std::fs::write("./pipeline.json", include_str!("defaults/pipeline.json")).unwrap();
        //std::fs::write("./fs.glsl", include_str!("defaults/fs.glsl")).unwrap();
        //std::fs::write("./vs.glsl", include_str!("defaults/vs.glsl")).unwrap();
        //return;
    }

    let mut jockey = Jockey::init();

    let pipeline_file = File::open("pipeline.json").expect("could not open pipeline file");

    jockey.update_pipeline(pipeline_file);

    println!("{:#?}", jockey.pipeline);

    let mut last_frame = Instant::now();
    let start_time = Instant::now();

    let vs = compile_shader(VS_SRC, gl::VERTEX_SHADER);
    let fs = compile_shader(FS_SRC, gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    'running: loop {
        for event in jockey.event_pump.poll_iter() {
            jockey.imgui_sdl2.handle_event(&mut jockey.imgui, &event);

            if jockey.imgui_sdl2.ignore_event(&event) {
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

        jockey.imgui_sdl2.prepare_frame(
            jockey.imgui.io_mut(),
            &jockey.window,
            &jockey.event_pump.mouse_state(),
        );

        let now = Instant::now();
        jockey.imgui.io_mut().delta_time = (now - last_frame).as_secs_f32();
        last_frame = now;

        // compute uniforms
        let (width, height) = jockey.window.size();
        let time = start_time.elapsed().as_secs_f32();

        // run all shader stages
        jockey.draw(width as _, height as _, time);

        /*
        unsafe {
            //gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            //gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::BindVertexArray(jockey.vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::BindBuffer(gl::ARRAY_BUFFER, jockey.vao);
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

            gl::DrawArrays(gl::TRIANGLES, 0, 6);
        }
        */

        let ui = jockey.imgui.frame();
        ui.show_demo_window(&mut true); // Zhe magic

        jockey.imgui_sdl2.prepare_render(&ui, &jockey.window);
        jockey.renderer.render(ui);

        jockey.window.gl_swap_window();

        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
