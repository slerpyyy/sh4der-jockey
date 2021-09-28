use std::{collections::HashMap, rc::Rc};

use gl::types::*;

use crate::*;
use crate::{jockey::Uniform, util::Geometry};

use super::Texture;

#[derive(std::fmt::Debug)]
pub struct Mesh {
    pub geometry: Geometry,
    pub uniforms: HashMap<*const GLchar, Uniform>,
    pub textures: HashMap<*const GLchar, Rc<dyn Texture>>,
}

impl Mesh {
    /// Bind uniforms and textures to given program.
    pub fn apply_uniforms(&self, program_id: GLuint, texture_unit: &mut GLuint) {
        for (name, uniform) in &self.uniforms {
            let location = unsafe { gl::GetUniformLocation(program_id, *name) };
            uniform.bind(location);
        }

        for (name, texture) in &self.textures {
            let location = unsafe { gl::GetUniformLocation(program_id, *name) };

            unsafe {
                gl::ActiveTexture(gl::TEXTURE0 + *texture_unit as GLenum);
                gl_debug_check!();

                texture.bind(*texture_unit);
                gl_debug_check!();

                gl::Uniform1i(location, *texture_unit as _);
                gl_debug_check!();
            }

            *texture_unit += 1;
        }
    }
}
