use gl::types::*;

mod uniformable_1f;
mod uniformable_4f;
mod uniformable_matrix_4fv;

pub use uniformable_1f::*;
pub use uniformable_4f::*;
pub use uniformable_matrix_4fv::*;

pub trait Uniformable: std::fmt::Debug {
    fn uniform(&self, location: GLint) -> ();
}
