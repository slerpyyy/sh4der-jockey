use glium::uniforms::{UniformValue, Uniforms};

pub struct UniformVec<'x> {
    inner: Vec<(&'x str, UniformValue<'x>)>,
}

impl<'x> UniformVec<'x> {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    pub fn add(&mut self, name: &'x str, value: UniformValue<'x>) {
        self.inner.push((name, value))
    }
}

//impl Uniforms for UniformVec<'_> {
//    fn visit_values<'a, F: FnMut(&str, UniformValue<'a>)>(&'a self, f: F) {
//        for (name, value) in self.inner.iter() {
//            f(name, value)
//        }
//    }
//}
