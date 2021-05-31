#![allow(dead_code)]
use crate::util::*;
use crate::*;
use as_any::AsAny;
use core::panic;
use image::DynamicImage;
use serde_yaml::Value;
use std::{fmt::Debug, rc::Rc, u8};

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
    pub fn new(width: u32, height: u32) -> Self {
        Self::with_more_params(
            width,
            height,
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            false,
            false,
        )
    }

    pub fn with_more_params(
        width: u32,
        height: u32,
        min_filter: GLenum,
        mag_filter: GLenum,
        wrap_mode: GLenum,
        mipmap: bool,
        float: bool,
    ) -> Self {
        let width = width.max(1);
        let height = height.max(1);

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

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, min_filter as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, mag_filter as _);
            gl_debug_check!();

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap_mode as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap_mode as _);
            gl_debug_check!();

            let (internal_format, type_) = match float {
                true => (gl::RGBA32F, gl::FLOAT),
                false => (gl::RGBA8, gl::UNSIGNED_BYTE),
            };

            gl::TexImage2D(
                gl::TEXTURE_2D,
                0,
                internal_format as _,
                width as _,
                height as _,
                0,
                gl::RGBA,
                type_,
                std::ptr::null(),
            );
            gl_debug_check!();

            if mipmap {
                gl::GenerateMipmap(gl::TEXTURE_2D);
            }

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

#[derive(Debug, Clone)]
pub struct TextureBuilder {
    pub resolution: Vec<u32>,
    pub min_filter: GLenum,
    pub mag_filter: GLenum,
    pub wrap_mode: GLenum,
    pub channels: u8,
    pub float: bool,
    pub mipmap: bool,
}

impl TextureBuilder {
    pub fn new() -> Self {
        Self {
            resolution: Vec::new(),
            min_filter: gl::NEAREST,
            mag_filter: gl::NEAREST,
            wrap_mode: gl::CLAMP_TO_EDGE,
            channels: 4,
            float: false,
            mipmap: false,
        }
    }

    pub fn parse(object: &Value, support_res: bool, support_mipmap: bool) -> Result<Self, String> {
        // get target resolution
        let resolution = match object
            .get("size")
            .or_else(|| object.get("res"))
            .or_else(|| object.get("resolution"))
            .filter(|_| support_res)
        {
            Some(Value::Sequence(dims)) => {
                if dims.is_empty() || dims.len() > 3 {
                    return Err(format!(
                        "Field \"resolution\" must be a list of 1 to 3 numbers, got {} elements",
                        dims.len()
                    ));
                }

                let mut out = Vec::with_capacity(3);
                for dim in dims {
                    match dim.as_u64() {
                        None => {
                            return Err(format!(
                            "Expected \"resolution\" to be a list of positive numbers, got {:?}",
                            dims
                        ))
                        }

                        Some(0) => {
                            return Err(format!(
                                "Expected all numbers in \"resolution\" to be positive, got {:?}",
                                dims
                            ))
                        }

                        Some(n) => out.push(n as _),
                    };
                }

                Some(out)
            }
            _ => None,
        }
        .unwrap_or_else(Vec::new);

        // get mipmap flag
        let mipmap = match object
            .get("mipmap")
            .map(Value::as_bool)
            .filter(|_| support_mipmap)
        {
            Some(Some(flag)) => flag,
            None => false,
            Some(s) => return Err(format!("Expected \"mipmap\" to be a bool, got {:?}", s)),
        };

        // get texture filtering mode
        let wrap_mode = match object
            .get("wrap_mode")
            .or_else(|| object.get("wrap"))
            .map(Value::as_str)
        {
            Some(Some("clamp")) | None => gl::CLAMP_TO_EDGE,
            Some(Some("repeat")) => gl::REPEAT,
            Some(Some("mirror")) => gl::MIRRORED_REPEAT,
            Some(s) => {
                return Err(format!(
                    "Expected \"wrap\" to be either \"repeat\", \"clamp\" or \"mirror\", got {:?}",
                    s
                ))
            }
        };

        // get texture filtering mode
        let mag_filter = match object.get("filter").map(Value::as_str) {
            Some(Some("linear")) | None => gl::LINEAR,
            Some(Some("nearest")) => gl::NEAREST,
            Some(s) => {
                return Err(format!(
                    "Expected \"filter\" to be either \"linear\" or \"nearest\", got {:?}",
                    s
                ))
            }
        };

        let min_filter = match (mag_filter, mipmap) {
            (gl::LINEAR, true) => gl::LINEAR_MIPMAP_LINEAR,
            (gl::NEAREST, true) => gl::NEAREST_MIPMAP_NEAREST,
            (filter, false) => filter,
            _ => unreachable!(),
        };

        // get float format flag
        let float = match object.get("float").map(Value::as_bool) {
            Some(Some(flag)) => flag,
            None => false,
            Some(s) => return Err(format!("Expected \"float\" to be a bool, got {:?}", s)),
        };

        Ok(Self {
            resolution,
            min_filter,
            mag_filter,
            wrap_mode,
            channels: 4,
            float,
            mipmap,
        })
    }

    pub fn set_resolution(&mut self, resolution: Vec<u32>) -> &mut Self {
        self.resolution = resolution;
        self
    }

    pub fn set_channels(&mut self, channels: u8) -> &mut Self {
        self.channels = channels;
        self
    }

    pub fn set_float(&mut self, is_float: bool) -> &mut Self {
        self.float = is_float;
        self
    }

    pub fn build_framebuffer(&self, screen_size: (u32, u32)) -> Rc<FrameBuffer> {
        let [width, height] = match self.resolution.as_slice() {
            &[w, h] => [w, h],
            &[] => [screen_size.0, screen_size.1],
            _ => unreachable!(),
        };

        Rc::new(FrameBuffer::with_more_params(
            width,
            height,
            self.min_filter,
            self.mag_filter,
            self.wrap_mode,
            self.mipmap,
            self.float,
        ))
    }

    fn texture_format(&self) -> TextureFormat {
        match (self.channels, self.float) {
            (1, false) => TextureFormat::R8,
            (2, false) => TextureFormat::RG8,
            (3, false) => TextureFormat::RGB8,
            (4, false) => TextureFormat::RGBA8,
            (1, true) => TextureFormat::R32F,
            (2, true) => TextureFormat::RG32F,
            (3, true) => TextureFormat::RGB32F,
            (4, true) => TextureFormat::RGBA32F,
            _ => unreachable!(),
        }
    }

    pub fn build_texture(&self) -> Rc<dyn Texture> {
        self.build_texture_with_data(std::ptr::null())
    }

    pub fn build_image(&self) -> Rc<dyn Texture> {
        self.build_image_with_data(std::ptr::null())
    }

    pub fn build_texture_with_data(&self, data: *const c_void) -> Rc<dyn Texture> {
        let format = self.texture_format();
        match self.resolution.as_slice() {
            &[w] => Rc::new(Texture1D::with_params(
                [w],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            &[w, h] => Rc::new(Texture2D::with_params(
                [w, h],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            &[w, h, d] => Rc::new(Texture3D::with_params(
                [w, h, d],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            _ => unreachable!(),
        }
    }

    pub fn build_image_with_data(&self, data: *const c_void) -> Rc<dyn Texture> {
        let format = self.texture_format();
        match self.resolution.as_slice() {
            &[w] => Rc::new(Image1D::with_params(
                [w],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            &[w, h] => Rc::new(Image2D::with_params(
                [w, h],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            &[w, h, d] => Rc::new(Image3D::with_params(
                [w, h, d],
                self.min_filter,
                self.mag_filter,
                self.wrap_mode,
                format,
                data,
            )),
            _ => unreachable!(),
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
pub enum TextureFormat {
    R8 = gl::R8 as _,
    RG8 = gl::RG8 as _,
    RGB8 = gl::RGB8 as _,
    RGBA8 = gl::RGBA8 as _,
    R32F = gl::R32F as _,
    RG32F = gl::RG32F as _,
    RGB32F = gl::RGB32F as _,
    RGBA32F = gl::RGBA32F as _,
}

macro_rules! impl_texture {
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
            pub fn new(resolution: [u32; $dim]) -> Self {
                Self::with_params(
                    resolution,
                    gl::LINEAR,
                    gl::LINEAR,
                    gl::REPEAT,
                    TextureFormat::RGBA32F,
                    std::ptr::null(),
                )
            }

            pub fn get_formats(format: TextureFormat) -> (i32, u32, u32) {
                let color_format = match format {
                    TextureFormat::R8 | TextureFormat::R32F => gl::RED,
                    TextureFormat::RG8 | TextureFormat::RG32F => gl::RG,
                    TextureFormat::RGB8 | TextureFormat::RGB32F => gl::RGB,
                    TextureFormat::RGBA32F | TextureFormat::RGBA8 => gl::RGBA,
                };

                let type_ = match format {
                    TextureFormat::R8
                    | TextureFormat::RG8
                    | TextureFormat::RGB8
                    | TextureFormat::RGBA8 => gl::UNSIGNED_BYTE,
                    TextureFormat::R32F
                    | TextureFormat::RG32F
                    | TextureFormat::RGB32F
                    | TextureFormat::RGBA32F => gl::FLOAT,
                };

                (format as i32, color_format as u32, type_ as u32)
            }

            pub fn with_params(
                mut resolution: [u32; $dim],
                min_filter: GLenum,
                mag_filter: GLenum,
                wrap_mode: GLenum,
                format: TextureFormat,
                data: *const c_void,
            ) -> Self {
                for k in resolution.iter_mut() {
                    *k = 1.max(*k);
                }

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
                        data,
                    );
                    gl_debug_check!();

                    if $is_image {
                        gl::BindImageTexture(
                            0,
                            tex_id,
                            0,
                            gl::FALSE,
                            0,
                            gl::READ_WRITE,
                            gl::RGBA32F,
                        );
                        gl_debug_check!();
                    }

                    Self {
                        id: tex_id,
                        format,
                        res: resolution,
                    }
                }
            }

            pub fn write(&mut self, data: *const c_void) {
                unsafe {
                    gl::BindTexture($enum_type, self.id);
                    gl_debug_check!();

                    let (internal_format, color_format, type_) = Self::get_formats(self.format);
                    gl_TexImageND(
                        $enum_type,
                        0,
                        internal_format,
                        &self.res,
                        0,
                        color_format,
                        type_,
                        data,
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

impl_texture!(Image1D, gl::TEXTURE_1D, 1, true);
impl_texture!(Image2D, gl::TEXTURE_2D, 2, true);
impl_texture!(Image3D, gl::TEXTURE_3D, 3, true);
impl_texture!(Texture1D, gl::TEXTURE_1D, 1, false);
impl_texture!(Texture2D, gl::TEXTURE_2D, 2, false);
impl_texture!(Texture3D, gl::TEXTURE_3D, 3, false);

#[deprecated]
pub fn make_image(resolution: &[u32]) -> Rc<dyn Texture> {
    match resolution {
        &[w] => Rc::new(Image1D::new([w])),
        &[w, h] => Rc::new(Image2D::new([w, h])),
        &[w, h, d] => Rc::new(Image3D::new([w, h, d])),
        _ => unreachable!(),
    }
}

#[deprecated]
pub fn make_texture(resolution: &[u32]) -> Rc<dyn Texture> {
    match resolution {
        &[w] => Rc::new(Texture1D::new([w])),
        &[w, h] => Rc::new(Texture2D::new([w, h])),
        &[w, h, d] => Rc::new(Texture3D::new([w, h, d])),
        _ => unreachable!(),
    }
}

pub fn make_noise() -> Texture3D {
    const WIDTH: usize = 32;
    const SIZE: usize = 4 * WIDTH * WIDTH * WIDTH;
    let data: Vec<u8> = (0..SIZE).map(|_| rand::random()).collect();
    let tex = Texture3D::with_params(
        [WIDTH as _; 3],
        gl::LINEAR,
        gl::LINEAR,
        gl::REPEAT,
        TextureFormat::RGBA8,
        data.as_ptr() as _,
    );

    tex
}

#[deprecated]
pub fn make_texture_from_image(dyn_image: DynamicImage) -> Texture2D {
    let image = dyn_image.flipv().to_rgba8();
    let tex = Texture2D::with_params(
        [image.width(), image.height()],
        gl::LINEAR,
        gl::LINEAR,
        gl::REPEAT,
        TextureFormat::RGBA8,
        image.as_raw().as_ptr() as _,
    );

    tex
}
