use gl::types::GLint;

use super::Uniformable;

pub struct Uniformable1f {
    pub value: f32,
}

impl Uniformable1f {
    pub fn new(value: f32) -> Self {
        Uniformable1f { value }
    }
}

impl std::fmt::Debug for Uniformable1f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Uniformable1f))
            .field("value", &self.value)
            .finish()
    }
}

impl Uniformable for Uniformable1f {
    fn uniform(&self, location: GLint) {
        unsafe { gl::Uniform1f(location, self.value); }
    }
}
