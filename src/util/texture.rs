use gl::types::*;

#[derive(Debug)]
pub enum TextureKind {
    FrameBuffer { fb: GLuint, resolution: (u32, u32) },
    Image1D { resolution: u32 },
    Image2D { resolution: (u32, u32) },
    Image3D { resolution: (u32, u32, u32) },
}

#[derive(Debug)]
pub struct Texture {
    /// The id of the texture object
    pub id: GLuint,
    pub kind: TextureKind,
}

impl Texture {
    pub fn with_framebuffer(width: GLsizei, height: GLsizei) -> Self {
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

            gl::TexStorage2D(gl::TEXTURE_2D, 4, gl::RGBA32F, width as _, height as _);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                4,
                0,
                0,
                width as _,
                height as _,
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

            Self {
                id,
                kind: TextureKind::FrameBuffer {
                    fb,
                    resolution: (width, height),
                },
            }
        }
    }

    pub fn new(resolution: &[u32]) -> Self {
        unsafe {
            let mut tex_id = 0;

            gl::GenTextures(1, &mut tex_id);
            gl::ActiveTexture(gl::TEXTURE0);

            match resolution {
                &[_, _, _] => todo!(),

                &[width, height] => {
                    gl::BindTexture(gl::TEXTURE_2D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        gl::RGBA32F as _,
                        width as _,
                        height as _,
                        0,
                        gl::RGBA,
                        gl::FLOAT,
                        std::ptr::null(),
                    );
                    TextureKind::Image2D {
                        resolution: (width, height),
                    }
                }

                &[width] => {
                    gl::BindTexture(gl::TEXTURE_1D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
                    gl::TexImage1D(
                        gl::TEXTURE_1D,
                        0,
                        gl::RGBA32F as _,
                        width as _,
                        0,
                        gl::RGBA,
                        gl::FLOAT,
                        std::ptr::null(),
                    );
                    TextureKind::Image1D { resolution: width }
                }

                s => panic!("Invalid texture resolution: {:?}", s),
            }

            gl::BindImageTexture(0, tex_id, 0, gl::FALSE, 0, gl::READ_WRITE, gl::RGBA32F);

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

            if let Some(fb) = self.fb {
                gl::DeleteFramebuffers(1, &fb)
            }
        }
    }
}
