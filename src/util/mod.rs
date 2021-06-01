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

#[macro_export]
macro_rules! gl_ignore {
    () => {
        // this unsafe in unnecessary if the macro is used in an unsafe block
        #[allow(unused_unsafe)]
        while (unsafe { gl::GetError() }) != gl::NO_ERROR {}
    };
}

#[macro_export]
macro_rules! gl_debug_ignore {
    () => {
        if cfg!(debug_assertions) {
            gl_ignore!();
        }
    };
}

const FULLSCREEN_TRI: [GLfloat; 6] = [-1.0, -1.0, 3.0, -1.0, -1.0, 3.0];

pub fn draw_fullscreen_tri(vao: GLuint) {
    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vao);
        gl_debug_check!();

        let data_size = FULLSCREEN_TRI.len() * std::mem::size_of::<GLfloat>();
        gl::BufferData(
            gl::ARRAY_BUFFER,
            data_size as _,
            std::mem::transmute(&FULLSCREEN_TRI[0]),
            gl::STATIC_DRAW,
        );
        gl_debug_check!();

        let vert_count = FULLSCREEN_TRI.len() as GLsizei / 2;
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

pub fn process_error(mut err: String, lut: &[String]) -> String {
    for (k, file) in lut.iter().enumerate() {
        let key = format!("{}", k + 101);
        err = err.replace(key.as_str(), file);
    }

    err
}

pub fn preprocess(
    code: &str,
    file_name: &str,
    file_name_lut: &mut Vec<String>,
) -> Result<String, String> {
    lazy_static! {
        // based on the "glsl-include" crate, which almost does what we want
        static ref INCLUDE_RE: Regex = Regex::new(
            r#"#\s*(pragma\s*)?include\s+[<"](?P<file>.*)[>"]"#
        ).expect("failed to compile regex");

        static ref ONCE_RE: Regex = Regex::new(
            r#"#\s*pragma\s+once"#
        ).expect("failed to compile regex");
    }

    fn recurse(
        code: &str,
        src_name: &str,
        mut cycle_seen: HashSet<String>,
        once_ignore: &mut HashSet<String>,
        lut: &mut Vec<String>,
    ) -> Result<Vec<String>, String> {
        let mut lines = Vec::<String>::new();
        let mut need_ln = true;

        // register file name
        let file_id = match lut.iter().position(|s| s == src_name) {
            Some(id) => id,
            None => {
                let index = lut.len();
                lut.push(src_name.into());
                index
            }
        };

        // offset file id
        #[cfg(not(test))]
        let file_id = file_id + 101;

        // respect pragma once
        let once_re: &Regex = &ONCE_RE;
        if let Some(once) = once_re.find(code) {
            let prefix = &code[..once.start()];
            if !once_ignore.insert(src_name.into())
                && !in_block(prefix, "//", "\n")
                && !in_block(prefix, "/*", "*/")
            {
                return Ok(Vec::new());
            }
        }

        // detect include cycles
        if !cycle_seen.insert(src_name.into()) {
            return Err(format!(
                "Cycle detected! File {} has been included further down the tree",
                src_name
            ));
        }

        // process code line by line
        for (k, line) in code.lines().enumerate() {
            let include_re: &Regex = &INCLUDE_RE;
            if let Some(include) = include_re.find(line) {
                let file_name = include_re
                    .captures(include.as_str())
                    .unwrap()
                    .name("file")
                    .unwrap()
                    .as_str();

                // get line prefix
                let offset = unsafe { include.as_str().as_ptr().offset_from(code.as_ptr()) };
                let prefix = &code[..offset as usize];

                // check for comments
                if !(in_block(prefix, "//", "\n") || in_block(prefix, "/*", "*/")) {
                    // fetch file
                    #[cfg(not(test))]
                    let file = match std::fs::read_to_string(file_name) {
                        Ok(s) => s,
                        Err(e) => return Err(e.to_string()),
                    };

                    // dummy for unit tests
                    #[cfg(test)]
                    let file = "#pragma once\nint hoge = 0;\n".to_string();

                    // recursively process file
                    let mut file_lines =
                        recurse(&file, file_name, cycle_seen.clone(), once_ignore, lut)?;
                    lines.append(&mut file_lines);

                    // put line directive above next line
                    need_ln = true;

                    // skip current line
                    continue;
                }
            }

            // add line directive
            if need_ln && !line.starts_with("#version") {
                lines.push(format!("#line {} {}", k, file_id));
                need_ln = false;
            }

            // add line
            lines.push(line.into());
        }

        Ok(lines)
    }

    // handle includes recursively
    let mut once_ignore = HashSet::new();
    let lines = recurse(
        &code,
        file_name,
        HashSet::new(),
        &mut once_ignore,
        file_name_lut,
    )?;
    Ok(lines.join("\n"))
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
        let expected = "#version 123\n#line 1 0\nmain(){}";
        let mut lut = Vec::new();
        let result = preprocess(original, "test", &mut lut).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_simple() {
        let original = "#version 123\n#pragma include \"foo.glsl\"\nmain(){}";
        let expected = "#version 123\n#line 0 1\n#pragma once\nint hoge = 0;\n#line 2 0\nmain(){}";
        let mut lut = Vec::new();
        let result = preprocess(original, "test", &mut lut).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_in_comment_single() {
        let original = "#version 123\n//#pragma include \"foo.glsl\"\nmain(){}";
        let expected = "#version 123\n#line 1 0\n//#pragma include \"foo.glsl\"\nmain(){}";
        let mut lut = Vec::new();
        let result = preprocess(original, "test", &mut lut).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_in_comment_block() {
        let original = "#version 123\n/*#pragma include \"foo.glsl\"*/\nmain(){}";
        let expected = "#version 123\n#line 1 0\n/*#pragma include \"foo.glsl\"*/\nmain(){}";
        let mut lut = Vec::new();
        let result = preprocess(original, "test", &mut lut).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn preprocess_include_pragma_once() {
        let original =
            "#version 123\n#pragma include \"foo.glsl\"\n#pragma include \"foo.glsl\"\nmain(){}";
        let expected = "#version 123\n#line 0 1\n#pragma once\nint hoge = 0;\n#line 3 0\nmain(){}";
        let mut lut = Vec::new();
        let result = preprocess(original, "test", &mut lut).unwrap();
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
