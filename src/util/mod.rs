use gl::types::*;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashSet,
    ffi::{c_void, CString},
};

mod average;
mod cache;
mod ringbuffer;
mod texture;

pub use average::*;
pub use cache::*;
pub use ringbuffer::*;
pub use texture::*;

#[macro_export]
macro_rules! gl_check {
    () => {
        // this unsafe in unnecessary if the macro is used in an unsafe block
        #[allow(unused_unsafe)]
        let err = unsafe { gl::GetError() };

        if err != gl::NO_ERROR {
            let name = match err {
                gl::INVALID_ENUM => "INVALID_ENUM",
                gl::INVALID_VALUE => "INVALID_VALUE",
                gl::INVALID_OPERATION => "INVALID_OPERATION",
                gl::INVALID_FRAMEBUFFER_OPERATION => "INVALID_ENUM",
                gl::OUT_OF_MEMORY => "OUT_OF_MEMORY",
                _ => "unknown",
            };

            panic!("OpenGL error: {} ({})", name, err);
        }
    };
}

#[macro_export]
macro_rules! gl_debug_check {
    () => {
        if cfg!(debug_assertions) {
            gl_check!();
        }
    };
}

const FULLSCREEN_RECT: [GLfloat; 12] = [
    -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0,
];

pub fn draw_fullscreen_rect(vao: GLuint) {
    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vao);
        gl_debug_check!();

        let data_size = FULLSCREEN_RECT.len() * std::mem::size_of::<GLfloat>();
        gl::BufferData(
            gl::ARRAY_BUFFER,
            data_size as _,
            std::mem::transmute(&FULLSCREEN_RECT[0]),
            gl::STATIC_DRAW,
        );
        gl_debug_check!();

        let vert_count = FULLSCREEN_RECT.len() as GLsizei / 2;
        gl::DrawArrays(gl::TRIANGLES, 0, vert_count);
        gl_debug_check!();
    }
}

pub fn draw_anything(vao: GLuint, count: GLsizei, mode: GLenum) {
    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vao);
        gl_debug_check!();

        gl::BufferData(gl::ARRAY_BUFFER, 0, std::ptr::null(), gl::STATIC_DRAW);
        gl_debug_check!();

        gl::DrawArrays(mode, 0, count);
        gl_debug_check!();
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

#[allow(non_snake_case)]
pub unsafe fn gl_TexImageND(
    target: GLenum,
    level: GLint,
    internalformat: GLint,
    resolution: &[u32],
    border: GLint,
    format: GLenum,
    type_: GLenum,
    pixels: *const c_void,
) {
    match target {
        gl::TEXTURE_1D => gl::TexImage1D(
            target,
            level,
            internalformat,
            resolution[0] as _,
            border,
            format,
            type_,
            pixels,
        ),
        gl::TEXTURE_2D => gl::TexImage2D(
            target,
            level,
            internalformat,
            resolution[0] as _,
            resolution[1] as _,
            border,
            format,
            type_,
            pixels,
        ),
        gl::TEXTURE_3D => gl::TexImage3D(
            target,
            level,
            internalformat,
            resolution[0] as _,
            resolution[1] as _,
            resolution[2] as _,
            border,
            format,
            type_,
            pixels,
        ),
        _ => unreachable!(),
    }
}

fn in_block(prefix: &str, start: &str, end: &str) -> bool {
    debug_assert_ne!(start, end);

    let start_opt = prefix.rfind(start);
    let end_opt = prefix.rfind(end);

    match (start_opt, end_opt) {
        (Some(s), Some(e)) => s + start.len() > e,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

pub fn preprocess(code: &str, file_name: &str) -> Result<String, String> {
    lazy_static! {
        // based on the "glsl-include" crate, which almost does what we want
        static ref INCLUDE_RE: Regex = Regex::new(
            r#"#\s*(pragma\s*)?include\s+[<"](?P<file>.*)[>"]"#
        ).expect("failed to compile regex");
    }

    fn recurse(code: &str, src_name: &str, mut seen: HashSet<String>) -> Result<String, String> {
        let include_re: &Regex = &INCLUDE_RE;
        if let Some(include) = include_re.find(code) {
            let caps = include_re.captures(include.as_str()).unwrap();
            let file_name = caps.name("file").unwrap().as_str();
            let leading_lines = code[0..include.start()]
                .bytes()
                .filter(|&byte| byte == b'\n')
                .count()
                .saturating_sub(1);

            // compute substring before and after match
            let prefix = &code[..include.start()];
            let postfix = recurse(&code[include.end()..], src_name, seen.clone())?;

            // respect comments
            if in_block(prefix, "//", "\n") || in_block(prefix, "/*", "*/") {
                return Ok(format!("{}{}", &code[..include.end()], postfix));
            }

            // detect include cycles
            if !seen.insert(file_name.into()) {
                return Err(format!(
                    "Cycle detected! File {} has been included further down the tree",
                    file_name
                ));
            }

            // fetch file
            #[cfg(not(test))]
            let file = match std::fs::read_to_string(file_name) {
                Ok(s) => s,
                Err(e) => return Err(e.to_string()),
            };

            // dummy for unit tests
            #[cfg(test)]
            let file = "int hoge = 0;\n";

            // recursively process included file
            let file = recurse(&file, &file_name, seen)?;

            Ok(format!(
                "{}\n#line 0 \"{}\"\n{}\n#line {} \"{}\"\n{}",
                prefix, file_name, file, leading_lines, src_name, postfix
            ))
        } else {
            Ok(code.into())
        }
    }

    // insert #line directive after version
    let first_nl = code.find('\n').unwrap_or(0);
    let (prefix, postfix) = code.split_at(first_nl);
    let code = format!("{}\n#line 1 \"{}\"{}", prefix, file_name, postfix);

    // handle includes recursively
    recurse(&code, file_name, HashSet::new())
}

pub fn interlace<T: Clone>(mut first: &[T], mut second: &[T]) -> Vec<T> {
    let mut out = Vec::with_capacity(first.len() + second.len());
    while let (Some((fh, ft)), Some((sh, st))) = (first.split_first(), second.split_first()) {
        out.push(fh.clone());
        out.push(sh.clone());
        first = ft;
        second = st;
    }

    out.extend_from_slice(first);
    out.extend_from_slice(second);
    out
}

#[allow(dead_code)]
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
    fn in_block_simple() {
        assert!(in_block("aa ( bb", "(", ")"));
        assert!(in_block("( aa ) bb (", "(", ")"));

        assert!(!in_block("( aa ( bb )", "(", ")"));
        assert!(!in_block("aa bb", "(", ")"));
    }

    #[test]
    fn in_block_overlap() {
        assert!(in_block("(x)", "(x", "x)"));
        assert!(in_block("(xx)", "(xx", "xx)"));
        assert!(in_block("(xx)", "(xx", "x)"));
        assert!(in_block("(xx)", "(x", "xx)"));
        assert!(in_block("(xxx)", "(xx", "xx)"));

        assert!(!in_block("(xx)", "(x", "x)"));
        assert!(!in_block("(xxx)", "(xx", "x)"));
        assert!(!in_block("(xxx)", "(x", "xx)"));
    }

    #[test]
    fn interlace_simple() {
        let first = &[1, 2, 3, 4];
        let second = &[5, 6, 7, 8];
        let vec = interlace(first, second);

        assert_eq!(vec, &[1, 5, 2, 6, 3, 7, 4, 8]);
    }

    #[test]
    fn deinterlace_simple() {
        let slice = &[1, 5, 2, 6, 3, 7, 4, 8];
        let (first, second) = deinterlace(slice);

        assert_eq!(first, &[1, 2, 3, 4]);
        assert_eq!(second, &[5, 6, 7, 8]);
    }

    #[test]
    fn interlace_unbalanced() {
        let first = &[1, 2, 3];
        let second = &[4, 5, 6, 7, 8];
        let vec = interlace(first, second);

        assert_eq!(vec, &[1, 4, 2, 5, 3, 6, 7, 8]);
    }

    #[test]
    fn deinterlace_unbalanced() {
        let slice = &[1, 2, 3, 4, 5];
        let (first, second) = deinterlace(slice);

        assert_eq!(first, &[1, 3, 5]);
        assert_eq!(second, &[2, 4]);
    }

    #[test]
    fn preprocess_line_number() {
        let original = "#version 123\nmain(){}";
        let expected = "#version 123\n#line 1 \"test\"\nmain(){}";
        let result = preprocess(original, "test").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_simple() {
        let original = "#version 123\n#pragma include \"foo.glsl\"\nmain(){}";
        let expected = "#version 123\n#line 1 \"test\"\n\n#line 0 \"foo.glsl\"\nint hoge = 0;\n\n#line 1 \"test\"\n\nmain(){}";
        let result = preprocess(original, "test").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_in_comment_single() {
        let original = "#version 123\n//#pragma include \"foo.glsl\"\nmain(){}";
        let expected = "#version 123\n#line 1 \"test\"\n//#pragma include \"foo.glsl\"\nmain(){}";
        let result = preprocess(original, "test").unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_in_comment_block() {
        let original = "#version 123\n/*#pragma include \"foo.glsl\"*/\nmain(){}";
        let expected = "#version 123\n#line 1 \"test\"\n/*#pragma include \"foo.glsl\"*/\nmain(){}";
        let result = preprocess(original, "test").unwrap();
        assert_eq!(result, expected);
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
