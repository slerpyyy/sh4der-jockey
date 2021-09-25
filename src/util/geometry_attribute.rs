use gl::types::*;

use crate::*;

/// A struct represents a single attribute of a Geometry.
#[derive(std::fmt::Debug)]
pub struct GeometryAttribute<T>
where
    T: std::fmt::Debug,
{
    /// The data of the attribute.
    pub array: Vec<T>,

    /// The item size of the attribute.
    pub size: GLuint,

    /// The type of the attribute.
    pub type_: GLenum,

    /// Whether the data is normalized or not.
    pub normalized: GLboolean,

    /// Either `gl::ARRAY_BUFFER` or `gl::ELEMENT_ARRAY_BUFFER`
    pub target: GLenum,

    /// Usage of the attribute.
    pub usage: GLenum,

    /// A buffer object for this attribute.
    buffer: Option<GLuint>,
}

impl<T> GeometryAttribute<T>
where
    T: std::fmt::Debug,
{
    pub fn init(array: Vec<T>, size: GLuint, type_: GLenum) -> Self {
        GeometryAttribute {
            array,
            size,
            type_,
            normalized: gl::FALSE,
            target: gl::ARRAY_BUFFER,
            usage: gl::STATIC_DRAW,
            buffer: None,
        }
    }

    /// Make a vertex buffer object out of this attribute and assign it to its buffer field.
    pub fn buffer(&mut self) -> GLuint {
        match self.buffer {
            None => {
                let mut buffer = 0;

                unsafe {
                    gl::GenBuffers(1, &mut buffer);
                    gl::BindBuffer(self.target, buffer);
                    gl_debug_check!();
                }

                unsafe {
                    gl::BufferData(
                        self.target,
                        (self.array.len() * std::mem::size_of::<T>()) as _,
                        std::mem::transmute(self.array.as_ptr()),
                        self.usage,
                    );
                    gl_debug_check!();
                }

                self.buffer = Some(buffer);

                buffer
            }
            Some(buffer) => buffer,
        }
    }

    pub fn vertex_attrib_pointer(&mut self, index: GLuint) {
        match self.buffer {
            None => (),
            Some(buffer) => {
                unsafe {
                    gl::BindBuffer(self.target, buffer);
                    gl_debug_check!();
                }

                unsafe {
                    gl::VertexAttribPointer(
                        index,
                        self.size as _,
                        self.type_,
                        self.normalized,
                        0,
                        std::ptr::null(),
                    );
                    gl_debug_check!();
                }
            }
        }
    }

    /// Delete the buffer object.
    pub fn delete_buffer(&mut self) {
        match self.buffer {
            None => (),
            Some(buffer) => {
                unsafe {
                    gl::DeleteBuffers(1, &buffer);
                    // gl_debug_check!();
                }

                self.buffer = None;
            }
        }
    }
}

impl<T> Drop for GeometryAttribute<T>
where
    T: std::fmt::Debug,
{
    fn drop(&mut self) {
        self.delete_buffer();
    }
}
