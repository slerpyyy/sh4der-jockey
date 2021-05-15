use crate::*;
use crate::util::gl_TexImageND;
use as_any::AsAny;
use core::panic;
use gl::types::*;
use std::fmt::Debug;

fn _assert_is_object_safe(_: &dyn Texture) {}

pub trait Texture: Debug + AsAny {
    fn activate(&self);
    fn resolution(&self) -> [u32; 3];
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub tex_id: GLuint,
    pub fb_id: GLuint,
    res: [u32; 2],
}

impl Texture for FrameBuffer {
    fn resolution(&self) -> [u32; 3] {
        [self.res[0], self.res[1], 0]
    }

    fn activate(&self) {
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.tex_id);
            gl_debug_check!();
        }
    }
}

impl FrameBuffer {
    #[allow(dead_code)]
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_params(width, height, false, true, true, true)
    }

    pub fn with_params(
        width: u32,
        height: u32,
        repeat: bool,
        linear: bool,
        mipmap: bool,
        float: bool,
    ) -> Self {
        unsafe {
            let mut tex_id = 0;
            let mut fb_id = 0;

            gl::GenTextures(1, &mut tex_id);
            gl::GenFramebuffers(1, &mut fb_id);
            gl_debug_check!();

            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex_id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb_id);
            gl_debug_check!();

            let (min, mag) = match (linear, mipmap) {
                (false, false) => (gl::NEAREST, gl::NEAREST),
                (false, true) => (gl::NEAREST_MIPMAP_NEAREST, gl::NEAREST),
                (true, false) => (gl::LINEAR, gl::LINEAR),
                (true, true) => (gl::LINEAR_MIPMAP_LINEAR, gl::LINEAR),
            };

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag as _);
            gl_debug_check!();

            let wrap = match repeat {
                true => gl::REPEAT,
                false => gl::CLAMP_TO_EDGE,
            };

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap as _);
            gl_debug_check!();

            let r#type = match float {
                true => gl::FLOAT,
                false => gl::UNSIGNED_BYTE,
            };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                gl::RGBA as _,
                width as _,
                height as _,
                0,
                gl::RGBA,
                r#type,
                std::ptr::null(),
            );
            gl_debug_check!();

            gl::GenerateMipmap(gl::TEXTURE_2D);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex_id,
                0,
            );

            gl_debug_check!();
            debug_assert_eq!(
                gl::CheckFramebufferStatus(gl::FRAMEBUFFER),
                gl::FRAMEBUFFER_COMPLETE
            );

            Self {
                tex_id,
                fb_id,
                res: [width, height],
            }
        }
    }
}

impl Drop for FrameBuffer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.tex_id);
            gl::DeleteFramebuffers(1, &self.fb_id);
        }
    }
}

#[derive(Debug)]
pub enum TextureKind {
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

macro_rules! impl_image {
    ($name:ident, $enum_type:expr, $dim:expr, $is_image:expr) => {
        #[derive(Debug)]
        pub struct $name {
            pub id: GLuint,
            pub format: TextureFormat,
            pub res: [u32; $dim],
        }

        impl Texture for $name {
            fn resolution(&self) -> [u32; 3] {
                let mut out = [0; 3];
                out.copy_from_slice(&self.res);
                out
            }

            fn activate(&self) {
                unsafe {
                    gl::BindTexture($enum_type, self.id);
                    gl_debug_check!();

                    if $is_image {
                        gl::BindImageTexture(
                            0,
                            self.id,
                            0,
                            gl::FALSE,
                            0,
                            gl::WRITE_ONLY,
                            self.format as _,
                        );
                        gl_debug_check!();
                    }
                }
            }
        }

        impl $name {
            #[allow(dead_code)]
            pub fn new(resolution: [u32; $dim]) -> Self {
                Self::with_params(
                    resolution,
                    gl::LINEAR,
                    gl::LINEAR,
                    gl::REPEAT,
                    TextureFormat::RGBA32F,
                )
            }

            #[allow(dead_code)]
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

            #[allow(dead_code)]
            pub fn with_params(
                resolution: [u32; $dim],
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

                    gl::BindTexture($enum_type, tex_id);
                    gl::TexParameteri($enum_type, gl::TEXTURE_MIN_FILTER, min_filter as _);
                    gl::TexParameteri($enum_type, gl::TEXTURE_MAG_FILTER, mag_filter as _);

                    gl::TexParameteri($enum_type, gl::TEXTURE_WRAP_S, wrap_mode as _);
                    if $dim > 1 {
                        gl::TexParameteri($enum_type, gl::TEXTURE_WRAP_T, wrap_mode as _);
                    }
                    if $dim > 2 {
                        gl::TexParameteri($enum_type, gl::TEXTURE_WRAP_R, wrap_mode as _);
                    }

                    gl_TexImageND(
                        $enum_type,
                        0,
                        internal_format,
                        &resolution,
                        0,
                        color_format,
                        type_,
                        std::ptr::null(),
                    );

                    if $is_image {
                        gl::BindImageTexture(0, tex_id, 0, gl::FALSE, 0, gl::READ_WRITE, gl::RGBA32F);
                        gl_debug_check!();
                    }

                    Self {
                        id: tex_id,
                        format,
                        res: resolution,
                    }
                }
            }

            #[allow(dead_code)]
            pub fn write(&mut self, data: &[f32]) {
                unsafe {
                    let tex_id = self.id;
                    let (internal_format, color_format, type_) = Self::get_formats(self.format);

                    gl::BindTexture($enum_type, tex_id);
                    gl_TexImageND(
                        $enum_type,
                        0,
                        internal_format,
                        &self.res,
                        0,
                        color_format,
                        type_,
                        data.as_ptr() as _,
                    );

                    gl_debug_check!();
                }
            }
        }

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe {
                    gl::DeleteTextures(1, &self.id);
                }
            }
        }
    };
}

impl_image!(Image1D, gl::TEXTURE_1D, 1, true);
impl_image!(Image2D, gl::TEXTURE_2D, 2, true);
impl_image!(Image3D, gl::TEXTURE_3D, 3, true);
impl_image!(Texture1D, gl::TEXTURE_1D, 1, false);
impl_image!(Texture2D, gl::TEXTURE_2D, 2, false);
impl_image!(Texture3D, gl::TEXTURE_3D, 3, false);

#[derive(Debug)]
pub struct ImageTexture {
    pub id: GLuint,
    pub kind: TextureKind,
    pub format: TextureFormat,
}

impl Texture for ImageTexture {
    fn resolution(&self) -> [u32; 3] {
        let mut out = [0; 3];

        match self.kind {
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

    fn activate(&self) {
        unsafe {
            match self.kind {
                TextureKind::Image2D { .. } | TextureKind::Texture2D { .. } => {
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
}

impl ImageTexture {
    pub fn new(resolution: &[u32]) -> Self {
        Self::with_params(
            resolution,
            gl::LINEAR,
            gl::LINEAR,
            gl::REPEAT,
            TextureFormat::RGBA32F,
        )
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

impl Drop for ImageTexture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
        }
    }
}
