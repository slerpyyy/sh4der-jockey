use gl::types::GLint;

use super::Uniformable;

#[derive(std::fmt::Debug)]
pub struct Uniformable4f {
    pub value: [f32; 4],
}

impl Uniformable4f {
    pub fn new(value: [f32; 4]) -> Self {
        Uniformable4f { value }
    }
}

impl Uniformable for Uniformable4f {
    fn uniform(&self, location: GLint) {
        unsafe {
            gl::Uniform4f(
                location,
                self.value[0],
                self.value[1],
                self.value[2],
                self.value[3],
            );
        }
    }
}
