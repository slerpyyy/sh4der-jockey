use gl::types::*;

#[derive(Debug)]
pub struct Texture {
    pub id: GLuint,
    pub fb: GLuint,
    pub slot: GLuint,
}

impl Texture {
    pub fn new(width: GLsizei, height: GLsizei, slot: GLuint) -> Self {
        unsafe {
            let mut id = 0;
            let mut fb = 0;

            gl::GenTextures(1, &mut id);
            gl::GenFramebuffers(1, &mut fb);

            gl::ActiveTexture(gl::TEXTURE0 + slot);
            gl::BindTexture(gl::TEXTURE_2D, id);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fb);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as _);

            #[rustfmt::skip]
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as _);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);

            gl::TexStorage2D(gl::TEXTURE_2D, 8, gl::RGBA32F, width, height);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
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

            Self { id, fb, slot }
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
            gl::DeleteFramebuffers(1, &self.fb);
        }
    }
}
