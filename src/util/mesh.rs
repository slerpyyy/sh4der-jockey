use std::collections::HashMap;

use gl::types::*;

use crate::util::{Geometry, Uniformable};

#[derive(std::fmt::Debug)]
pub struct Mesh {
    pub geometry: Geometry,
    pub uniforms: HashMap<*const GLchar, Box<dyn Uniformable>>,
}

impl Mesh {
    pub fn apply_uniforms(&self, program_id: GLuint) {
        for (name, uniformable) in &self.uniforms {
            let location = unsafe { gl::GetUniformLocation(program_id, *name) };
            uniformable.uniform(location);
        }
    }
}
