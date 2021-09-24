use std::{collections::HashMap};

use gl::types::*;

use super::{Geometry, Uniformable};

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

impl std::fmt::Debug for Mesh {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Mesh))
            .field("geometry", &self.geometry)
            .field("uniforms", &self.uniforms)
            .finish()
    }
}
