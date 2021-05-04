use gl::types::*;
use itertools::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::{collections::HashSet, ffi::CString};

mod average;
mod ringbuffer;
mod texture;

pub use average::*;
pub use ringbuffer::*;
pub use texture::*;

const FULLSCREEN_RECT: [GLfloat; 12] = [
    -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0,
];

pub fn draw_fullscreen_rect(vao: GLuint) {
    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vao);

        let data_size = FULLSCREEN_RECT.len() * std::mem::size_of::<GLfloat>();
        gl::BufferData(
            gl::ARRAY_BUFFER,
            data_size as _,
            std::mem::transmute(&FULLSCREEN_RECT[0]),
            gl::STATIC_DRAW,
        );

        let vert_count = FULLSCREEN_RECT.len() as GLsizei / 2;
        gl::DrawArrays(gl::TRIANGLES, 0, vert_count);
    }
}

pub fn draw_anything(vao: GLuint, count: GLsizei, mode: GLenum) {
    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vao);

        gl::BufferData(gl::ARRAY_BUFFER, 0, std::ptr::null(), gl::STATIC_DRAW);

        gl::DrawArrays(mode, 0, count);
    }
}

pub fn compile_shader(src: &str, ty: GLenum) -> Result<GLuint, String> {
    unsafe {
        let shader = gl::CreateShader(ty);

        // Attempt to compile the shader
        let c_str = CString::new(src.as_bytes()).unwrap();
        gl::ShaderSource(shader, 1, &c_str.as_ptr(), std::ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);

            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize).saturating_sub(1));

            gl::GetShaderInfoLog(shader, len, std::ptr::null_mut(), buf.as_mut_ptr() as _);

            let msg = std::str::from_utf8_unchecked(&buf);
            return Err(msg.into());
        }

        Ok(shader)
    }
}

/// Creates a program from a slice of shaders.
///
/// Creates a new program and attaches the given shaders to that program.
pub fn link_program(sh: &[GLuint]) -> Result<GLuint, String> {
    unsafe {
        let program = gl::CreateProgram();

        // Link program
        sh.iter().for_each(|&s| gl::AttachShader(program, s));
        gl::LinkProgram(program);

        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);

            let mut buf = Vec::with_capacity(len as usize);
            buf.set_len((len as usize).saturating_sub(1));

            gl::GetProgramInfoLog(program, len, std::ptr::null_mut(), buf.as_mut_ptr() as _);

            let msg = std::str::from_utf8_unchecked(&buf);
            return Err(msg.into());
        }

        Ok(program)
    }
}

pub fn gl_check() {
    unsafe {
        match gl::GetError() {
            gl::NO_ERROR => return,
            gl::INVALID_ENUM => panic!("OpenGL error INVALID_ENUM {}", gl::INVALID_ENUM),
            gl::INVALID_VALUE => panic!("OpenGL error INVALID_VALUE {}", gl::INVALID_VALUE),
            gl::INVALID_OPERATION => {
                panic!("OpenGL error INVALID_OPERATION {}", gl::INVALID_OPERATION)
            }
            gl::INVALID_FRAMEBUFFER_OPERATION => panic!(
                "OpenGL error INVALID_FRAMEBUFFER_OPERATION {}",
                gl::INVALID_FRAMEBUFFER_OPERATION
            ),
            gl::OUT_OF_MEMORY => panic!("OpenGL error OUT_OF_MEMORY {}", gl::OUT_OF_MEMORY),

            err => panic!("OpenGL error: {}", err),
        }
    }
}

pub fn preprocess(code: &str) -> Result<String, String> {
    lazy_static! {
        // based on the "glsl-include" crate, which almost does what we want
        static ref INCLUDE_RE: Regex = Regex::new(
            r#"#\s*(pragma\s*)?include\s+[<"](?P<file>.*)[>"]"#
        ).expect("failed to compile regex");
    }

    fn recurse(code: &str, mut seen: HashSet<String>) -> Result<String, String> {
        if let Some(include) = INCLUDE_RE.find(code) {
            let caps = INCLUDE_RE.captures(include.as_str()).unwrap();
            let file_name = caps.name("file").unwrap().as_str();

            // detect include cycles
            if !seen.insert(file_name.to_owned()) {
                return Err(format!(
                    "Cycle detected! File {} has been included further down the tree",
                    file_name
                ));
            }

            let file = match std::fs::read_to_string(file_name) {
                Ok(s) => s,
                Err(e) => return Err(e.to_string()),
            };

            let prefix = &code[..include.start()];
            let file = recurse(&file, seen.clone())?;
            let postfix = recurse(&code[include.end()..], seen)?;

            Ok(format!("{}{}{}", prefix, file, postfix))
        } else {
            Ok(code.to_owned())
        }
    }

    recurse(code, HashSet::new())
}

pub fn interlace<T: Clone>(first: &[T], second: &[T]) -> Vec<T> {
    first.iter().interleave(second).cloned().collect()
}

pub fn deinterlace<T: Clone>(slice: &[T]) -> (Vec<T>, Vec<T>) {
    (
        slice.iter().step_by(2).cloned().collect(),
        slice.iter().skip(1).step_by(2).cloned().collect(),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn interlace_simple() {
        let arr = [1, 2, 3, 4, 5, 6, 7, 8];
        let vec = interlace(&arr[..4], &arr[4..]);

        assert_eq!(vec, vec![1, 5, 2, 6, 3, 7, 4, 8]);
    }

    #[test]
    fn interlace_cycle() {
        let original = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let inter = interlace(&original[..4], &original[4..]);
        let (mut a, mut b) = deinterlace(&inter);
        a.append(&mut b);

        assert_eq!(a, original);
    }
}

#[allow(dead_code)]
pub fn test_compute_capabilities() {
    unsafe {
        let mut work_group_count_x = 0;
        let mut work_group_count_y = 0;
        let mut work_group_count_z = 0;
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 0, &mut work_group_count_x);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 1, &mut work_group_count_y);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_COUNT, 2, &mut work_group_count_z);

        println!(
            "Work group count: {:?}, {:?}, {:?}",
            work_group_count_x, work_group_count_y, work_group_count_z
        );
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 0, &mut work_group_count_x);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 1, &mut work_group_count_y);
        gl::GetIntegeri_v(gl::MAX_COMPUTE_WORK_GROUP_SIZE, 2, &mut work_group_count_z);
        println!(
            "Work group size: {:?}, {:?}, {:?}",
            work_group_count_x, work_group_count_y, work_group_count_z
        );

        let mut work_group_invocations = 0;
        gl::GetIntegerv(
            gl::MAX_COMPUTE_WORK_GROUP_INVOCATIONS,
            &mut work_group_invocations,
        );

        println!("Max work group invocations: {:?}", work_group_invocations);
    }
}

#[allow(dead_code)]
pub fn create_texture(width: GLsizei, height: GLsizei, index: GLuint) -> (GLuint, GLuint, GLuint) {
    unsafe {
        let mut tex = 0;
        let mut fb = 0;

        gl::GenTextures(1, &mut tex);
        gl::GenFramebuffers(1, &mut fb);

        gl::ActiveTexture(gl::TEXTURE0 + index);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::BindFramebuffer(gl::FRAMEBUFFER, fb);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

        gl::TexParameteri(
            gl::TEXTURE_2D,
            gl::TEXTURE_MIN_FILTER,
            gl::LINEAR_MIPMAP_LINEAR as i32,
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RGBA as _,
            width,
            height,
            0,
            gl::RGBA as _,
            gl::FLOAT,
            std::ptr::null(),
        );

        gl::GenerateMipmap(gl::TEXTURE_2D);

        gl::FramebufferTexture2D(
            gl::FRAMEBUFFER,
            gl::COLOR_ATTACHMENT0,
            gl::TEXTURE_2D,
            tex,
            0,
        );

        assert_eq!(
            gl::CheckFramebufferStatus(gl::FRAMEBUFFER),
            gl::FRAMEBUFFER_COMPLETE
        );

        (tex, fb, index)
    }
}
