use std::collections::HashMap;

use gl::types::*;

/// A struct represents a single attribute of a Geometry.
/// You should not use a single GeometryAttribute across multiple Geometries,
/// since Geometry will delete the gl buffer on drop.
#[derive(std::fmt::Debug)]
pub struct GeometryAttribute<T> where T: std::fmt::Debug {
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

/// A struct represents a geometry.
#[derive(std::fmt::Debug)]
pub struct Geometry {
    /// Count of vertices.
    pub count: GLsizei,

    /// Drawing mode e.g. `gl::TRIANGLES` .
    pub mode: GLenum,

    /// Attributes of the geometry. Keys are attribute location.
    pub attributes: HashMap<GLuint, GeometryAttribute<GLfloat>>,

    /// Index buffer of the geometry.
    pub indices: Option<GeometryAttribute<GLuint>>,

    /// A vao object for this geometry.
    vao: Option<GLuint>,
}

impl<T> GeometryAttribute<T> where T: std::fmt::Debug {
    pub fn init(
        array: Vec<T>,
        size: GLuint,
        type_: GLenum,
    ) -> Self {
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
            },
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

impl<T> Drop for GeometryAttribute<T> where T: std::fmt::Debug {
    fn drop(&mut self) {
        self.delete_buffer();
    }
}

impl Geometry {
    pub const ATTRIBUTE_POSITION: GLuint = 0;
    pub const ATTRIBUTE_NORMAL: GLuint = 1;
    pub const ATTRIBUTE_TEXCOORD0: GLuint = 2;

    pub fn init(count: GLsizei) -> Self {
        Geometry {
            count,
            mode: gl::TRIANGLES,
            attributes: HashMap::new(),
            indices: None,
            vao: None,
        }
    }

    /// Make a fullscreen rect geometry.
    pub fn fullscreen_rect() -> Self {
        let mut geometry = Geometry::init(4);
        geometry.mode = gl::TRIANGLE_STRIP;

        let attr_pos = GeometryAttribute::init(
            vec![-1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0],
            2,
            gl::FLOAT,
        );
        geometry.attributes.insert(0, attr_pos);

        geometry
    }

    /// Make a vertex array object out of this geometry and assign it to its vao field.
    pub fn vao(&mut self) -> GLuint {
        match self.vao {
            None => {
                // vao
                let mut vao = 0;

                unsafe {
                    gl::GenVertexArrays(1, &mut vao);
                    gl::BindVertexArray(vao);
                    gl_debug_check!();
                }

                // indices
                if let Some(indices) = &mut self.indices {
                    indices.buffer();
                }

                // attributes
                for (index, attribute) in self.attributes.iter_mut() {
                    attribute.buffer();

                    unsafe {
                        gl::EnableVertexAttribArray(*index);
                        gl_debug_check!();
                    }

                    attribute.vertex_attrib_pointer(*index);
                }

                self.vao = Some(vao);

                vao
            },
            Some(vao) => vao,
        }
    }

    /// Delete the vertex array object.
    pub fn delete_vao(&mut self) {
        match self.vao {
            None => (),
            Some(vao) => {
                unsafe {
                    gl::DeleteVertexArrays(1, &vao);
                    // gl_debug_check!();
                }

                self.vao = None;
            }
        }
    }
}

impl Drop for Geometry {
    fn drop(&mut self) {
        self.delete_vao();
    }
}
