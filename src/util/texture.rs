use crate::*;
use core::panic;
use gl::types::*;

#[derive(Debug)]
pub enum TextureKind {
    FrameBuffer { fb: GLuint, res: [u32; 2] },
    Image1D { res: [u32; 1] },
    Image2D { res: [u32; 2] },
    Image3D { res: [u32; 3] },
    Texture1D { res: [u32; 1] },
    Texture2D { res: [u32; 2] },
    Texture3D { res: [u32; 3] },
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum TextureFormat {
    R8I = gl::R8I as _,
    R32F = gl::R32F as _,
    RG8I = gl::RG8I as _,
    RG32F = gl::RG32F as _,
    RGB8I = gl::RGB8I as _,
    RGB32F = gl::RGB32F as _,
    RGBA8I = gl::RGBA8I as _,
    RGBA32F = gl::RGBA32F as _,
}

#[derive(Debug)]
pub struct Texture {
    pub id: GLuint,
    pub kind: TextureKind,
    pub format: TextureFormat,
}

impl Texture {
    pub fn new(resolution: &[u32]) -> Self {
        Self::with_params(
            resolution,
            gl::LINEAR,
            gl::LINEAR,
            gl::REPEAT,
            TextureFormat::RGBA32F,
        )
    }

    pub fn with_framebuffer(width: u32, height: u32) -> Self {
        unsafe {
            let mut id = 0;
            let mut fb = 0;

            gl::GenTextures(1, &mut id);
            gl::GenFramebuffers(1, &mut fb);
            gl_debug_check!();

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb);
            gl_debug_check!();

            #[rustfmt::skip]
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);
            gl_debug_check!();

            // gl::TexStorage2D(gl::TEXTURE_2D, 4, gl::RGBA32F, width as _, height as _);
            // gl::TexSubImage2D(
            //     gl::TEXTURE_2D,
            //     0,
            //     0,
            //     0,
            //     width as _,
            //     height as _,
            //     gl::RGBA,
            //     gl::FLOAT,
            //     std::ptr::null(),
            // );

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
            gl_debug_check!();

            gl::GenerateMipmap(gl::TEXTURE_2D);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                id,
                0,
            );

            gl_debug_check!();
            debug_assert_eq!(
                gl::CheckFramebufferStatus(gl::FRAMEBUFFER),
                gl::FRAMEBUFFER_COMPLETE
            );

            let format = TextureFormat::RGBA32F;
            Self {
                id,
                kind: TextureKind::FrameBuffer {
                    fb,
                    res: [width, height],
                },
                format,
            }
        }
    }

    pub fn get_formats(format: TextureFormat) -> (i32, u32, u32) {
        let color_format = match format {
            TextureFormat::R8I | TextureFormat::R32F => gl::RED,
            TextureFormat::RG8I | TextureFormat::RG32F => gl::RG,
            TextureFormat::RGB8I | TextureFormat::RGB32F => gl::RGB,
            TextureFormat::RGBA32F | TextureFormat::RGBA8I => gl::RGBA,
        };

        let type_ = match format {
            TextureFormat::R8I
            | TextureFormat::RG8I
            | TextureFormat::RGB8I
            | TextureFormat::RGBA8I => gl::INT,
            TextureFormat::R32F
            | TextureFormat::RG32F
            | TextureFormat::RGB32F
            | TextureFormat::RGBA32F => gl::FLOAT,
        };

        (format as i32, color_format as u32, type_ as u32)
    }

    pub fn with_params(
        resolution: &[u32],
        min_filter: GLenum,
        mag_filter: GLenum,
        wrap_mode: GLenum,
        format: TextureFormat,
    ) -> Self {
        unsafe {
            let mut tex_id = 0;

            gl::GenTextures(1, &mut tex_id);
            gl::ActiveTexture(gl::TEXTURE0);
            gl_debug_check!();

            let (internal_format, color_format, type_) = Self::get_formats(format);

            let kind = match resolution {
                &[width, height, depth] => {
                    gl::BindTexture(gl::TEXTURE_3D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_T, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_R, wrap_mode as _);
                    gl::TexImage3D(
                        gl::TEXTURE_3D,
                        0,
                        internal_format,
                        width as _,
                        height as _,
                        depth as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );
                    TextureKind::Image3D {
                        res: [width, height, depth],
                    }
                }

                &[width, height] => {
                    gl::BindTexture(gl::TEXTURE_2D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap_mode as _);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        internal_format,
                        width as _,
                        height as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );
                    TextureKind::Image2D {
                        res: [width, height],
                    }
                }

                &[width] => {
                    gl::BindTexture(gl::TEXTURE_1D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexImage1D(
                        gl::TEXTURE_1D,
                        0,
                        internal_format,
                        width as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );
                    TextureKind::Image1D { res: [width] }
                }

                s => panic!("Invalid texture resolution: {:?}", s),
            };

            gl::BindImageTexture(0, tex_id, 0, gl::FALSE, 0, gl::READ_WRITE, gl::RGBA32F);
            gl_debug_check!();

            Self {
                id: tex_id,
                kind,
                format,
            }
        }
    }

    pub fn texture_from_params(
        resolution: &[u32],
        min_filter: GLenum,
        mag_filter: GLenum,
        wrap_mode: GLenum,
        format: TextureFormat,
    ) -> Self {
        let mut tex_id = 0;
        unsafe {
            gl::GenTextures(1, &mut tex_id);
            gl::ActiveTexture(gl::TEXTURE0);
            gl_debug_check!();

            let (internal_format, color_format, type_) = Self::get_formats(format);

            let kind = match resolution {
                &[width, height, depth] => {
                    gl::BindTexture(gl::TEXTURE_3D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_T, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_3D, gl::TEXTURE_WRAP_R, wrap_mode as _);
                    gl::TexImage3D(
                        gl::TEXTURE_3D,
                        0,
                        internal_format,
                        width as _,
                        height as _,
                        depth as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );

                    gl::GenerateMipmap(gl::TEXTURE_3D);
                    TextureKind::Texture3D {
                        res: [width, height, depth],
                    }
                }
                &[width, height] => {
                    gl::BindTexture(gl::TEXTURE_2D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap_mode as _);
                    gl::TexImage2D(
                        gl::TEXTURE_2D,
                        0,
                        internal_format,
                        width as _,
                        height as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );

                    gl::GenerateMipmap(gl::TEXTURE_2D);
                    TextureKind::Texture2D {
                        res: [width, height],
                    }
                }
                &[width] => {
                    gl::BindTexture(gl::TEXTURE_1D, tex_id);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
                    gl::TexParameteri(gl::TEXTURE_1D, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    gl::TexImage1D(
                        gl::TEXTURE_1D,
                        0,
                        internal_format,
                        width as _,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );

                    gl::GenerateMipmap(gl::TEXTURE_1D);
                    TextureKind::Texture1D { res: [width] }
                }
                s => panic!("Invalid texture resolution: {:?}", s),
            };

            gl_debug_check!();
            Self {
                id: tex_id,
                kind,
                format,
            }
        }
    }

    pub fn resolution(&self) -> [u32; 3] {
        let mut out = [0; 3];

        match self.kind {
            TextureKind::FrameBuffer { res, .. } => out.copy_from_slice(&res),
            TextureKind::Image1D { res, .. } | TextureKind::Texture1D { res, .. } => {
                out.copy_from_slice(&res)
            }
            TextureKind::Image2D { res, .. } | TextureKind::Texture2D { res, .. } => {
                out.copy_from_slice(&res)
            }
            TextureKind::Image3D { res, .. } | TextureKind::Texture3D { res, .. } => {
                out.copy_from_slice(&res)
            }
        }

        out
    }

    pub fn activate(&self) {
        unsafe {
            match self.kind {
                TextureKind::FrameBuffer { .. }
                | TextureKind::Image2D { .. }
                | TextureKind::Texture2D { .. } => {
                    gl::BindTexture(gl::TEXTURE_2D, self.id);
                }
                TextureKind::Image1D { .. } | TextureKind::Texture1D { .. } => {
                    gl::BindTexture(gl::TEXTURE_1D, self.id);
                }
                TextureKind::Image3D { .. } | TextureKind::Texture3D { .. } => {
                    gl::BindTexture(gl::TEXTURE_3D, self.id);
                }
            };
            gl_debug_check!();

            match self.kind {
                TextureKind::Image1D { .. }
                | TextureKind::Image2D { .. }
                | TextureKind::Image3D { .. } => {
                    gl::BindImageTexture(
                        0,
                        self.id,
                        0,
                        gl::FALSE,
                        0,
                        gl::WRITE_ONLY,
                        self.format as _,
                    );
                }
                _ => (),
            };

            gl_debug_check!();
        }
    }

    pub fn write(&mut self, data: &[f32]) {
        unsafe {
            let tex_id = self.id;
            let (internal_format, color_format, type_) = Self::get_formats(self.format);
            match self.kind {
                TextureKind::Texture1D { res, .. } | TextureKind::Image1D { res, .. } => {
                    gl::BindTexture(gl::TEXTURE_1D, tex_id);
                    gl::TexImage1D(
                        gl::TEXTURE_1D,
                        0,
                        internal_format,
                        res[0] as _,
                        0,
                        color_format,
                        type_,
                        data.as_ptr() as _,
                    );
                }
                _ => todo!(),
            }
            gl_debug_check!();
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);

            if let TextureKind::FrameBuffer { fb, .. } = self.kind {
                gl::DeleteFramebuffers(1, &fb)
            }
        }
    }
}
