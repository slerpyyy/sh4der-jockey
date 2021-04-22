use gl::types::*;

#[derive(Debug)]
pub struct Texture {
    /// The id of the texture object
    pub id: GLuint,
    /// The id of the framebuffer which is attached to this texture
    pub fb: Option<GLuint>,
}

impl Texture {
    pub fn new(width: GLsizei, height: GLsizei) -> Self {
        unsafe {
            let mut id = 0;
            let mut fb = 0;

            gl::GenTextures(1, &mut id);
            gl::GenFramebuffers(1, &mut fb);

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            #[rustfmt::skip]
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

            gl::TexStorage2D(gl::TEXTURE_2D, 4, gl::RGBA32F, width, height);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                4,
                0,
                0,
                width,
                height,
                gl::RGBA32F,
                gl::FLOAT,
                std::ptr::null(),
            );

            gl::GenerateMipmap(gl::TEXTURE_2D);

            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                id,
                0,
            );

            assert_eq!(
                gl::CheckFramebufferStatus(gl::FRAMEBUFFER),
                gl::FRAMEBUFFER_COMPLETE
            );

            Self { id, fb: Some(fb) }
        }
    }

    pub fn create_image_texture(tex_type: GLuint, tex_dim: [u32; 3]) -> Self {
        unsafe {
            let mut tex_id = 0;
            match tex_type {
                gl::TEXTURE_3D => todo!(),
                gl::TEXTURE_2D => {
                    gl::GenTextures(1, &mut tex_id);
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_2D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
                    gl::TexStorage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA32F,
                        tex_dim[0] as _,
                        tex_dim[1] as _,
                    );
                    gl::TexSubImage2D(
                        gl::TEXTURE_2D,
                        4,
                        0,
                        0,
                        tex_dim[0] as _,
                        tex_dim[1] as _,
                        gl::RGBA32F,
                        gl::FLOAT,
                        std::ptr::null(),
                    );
                }
                gl::TEXTURE_1D => {
                    gl::GenTextures(1, &mut tex_id);
                    gl::ActiveTexture(gl::TEXTURE0);
                    gl::BindTexture(gl::TEXTURE_1D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
                    gl::TexStorage1D(gl::TEXTURE_1D, 0, gl::RGBA32F, tex_dim[0] as _);
                    gl::TexSubImage1D(
                        gl::TEXTURE_1D,
                        0,
                        0,
                        tex_dim[0] as _,
                        gl::RGBA32F,
                        gl::FLOAT,
                        std::ptr::null(),
                    );
                }
                _ => panic!("Expected texture type, got {:?}", tex_type),
            }

            gl::BindImageTexture(0, tex_id, 0, gl::FALSE, 0, gl::WRITE_ONLY, gl::RGBA32F);

            Self {
                id: tex_id,
                fb: None,
            }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
            match self.fb {
                Some(fb) => gl::DeleteFramebuffers(1, &fb),
                None => (),
            }
        }
    }
}
