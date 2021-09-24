use gl::types::GLint;

use super::Uniformable;

#[derive(std::fmt::Debug)]
pub struct Uniformable1f {
    pub value: f32,
}

impl Uniformable1f {
    pub fn new(value: f32) -> Self {
        Uniformable1f { value }
    }
}

impl Uniformable for Uniformable1f {
    fn uniform(&self, location: GLint) {
        unsafe { gl::Uniform1f(location, self.value); }
    }
}
