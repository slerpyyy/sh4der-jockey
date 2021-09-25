use gl::types::GLint;

use super::Uniformable;

#[derive(std::fmt::Debug)]
pub struct UniformableMatrix4fv {
    pub value: [[f32; 4]; 4],
}

impl UniformableMatrix4fv {
    pub fn new(value: [[f32; 4]; 4]) -> Self {
        UniformableMatrix4fv { value }
    }
}

impl Uniformable for UniformableMatrix4fv {
    fn uniform(&self, location: GLint) {
        unsafe {
            gl::UniformMatrix4fv(
                location,
                1,
                gl::FALSE,
                std::intrinsics::transmute(self.value.as_ptr()),
            );
        }
    }
}
