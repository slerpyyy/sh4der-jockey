use std::collections::HashMap;

use gl::types::*;

use crate::{jockey::Uniform, util::Geometry};

#[derive(std::fmt::Debug)]
pub struct Mesh {
    pub geometry: Geometry,
    pub uniforms: HashMap<*const GLchar, Uniform>,
}

impl Mesh {
    pub fn apply_uniforms(&self, program_id: GLuint) {
        for (name, uniform) in &self.uniforms {
            let location = unsafe { gl::GetUniformLocation(program_id, *name) };
            uniform.bind(location);
        }
    }
}
