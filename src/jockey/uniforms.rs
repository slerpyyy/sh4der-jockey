use std::ffi::CString;

use anyhow::{bail, Result};
use gl::types::*;
use lazy_static::lazy_static;

lazy_static! {
    // slerpys golf coding stuff
    pub static ref R_NAME: CString = CString::new("R").unwrap();
    pub static ref K_NAME: CString = CString::new("K").unwrap();

    // miscellaneous
    pub static ref RESOLUTION_NAME: CString = CString::new("resolution").unwrap();
    pub static ref PASS_INDEX_NAME: CString = CString::new("pass_index").unwrap();
    pub static ref OUT_COLOR_NAME: CString = CString::new("out_color").unwrap();
    pub static ref POSITION_NAME: CString = CString::new("position").unwrap();
    pub static ref VERTEX_COUNT_NAME: CString = CString::new("vertex_count").unwrap();
    pub static ref NOISE_NAME: CString = CString::new("noise").unwrap();

    // time tracking
    pub static ref TIME_NAME: CString = CString::new("time").unwrap();
    pub static ref TIME_DELTA_NAME: CString = CString::new("time_delta").unwrap();
    pub static ref FRAME_COUNT_NAME: CString = CString::new("frame_count").unwrap();

    // direct user input
    pub static ref BEAT_NAME: CString = CString::new("beat").unwrap();
    pub static ref SLIDERS_NAME: CString = CString::new("sliders").unwrap();
    pub static ref BUTTONS_NAME: CString = CString::new("buttons").unwrap();

    // volume input
    pub static ref VOLUME_NAME: CString = CString::new("volume").unwrap();
    pub static ref VOLUME_INTEGRATED_NAME: CString = CString::new("volume_integrated").unwrap();

    // audio textures
    pub static ref SAMPLES_NAME: CString = CString::new("samples").unwrap();
    pub static ref SPECTRUM_NAME: CString = CString::new("spectrum").unwrap();
    pub static ref SPECTRUM_RAW_NAME: CString = CString::new("spectrum_raw").unwrap();
    pub static ref SPECTRUM_SMOOTH_NAME: CString = CString::new("spectrum_smooth").unwrap();
    pub static ref SPECTRUM_INTEGRATED_NAME: CString = CString::new("spectrum_integrated").unwrap();
    pub static ref SPECTRUM_SMOOTH_INTEGRATED_NAME: CString = CString::new("spectrum_smooth_integrated").unwrap();

    // bass
    pub static ref BASS_NAME: CString = CString::new("bass").unwrap();
    pub static ref BASS_SMOOTH_NAME: CString = CString::new("bass_smooth").unwrap();
    pub static ref BASS_INTEGRATED_NAME: CString = CString::new("bass_integrated").unwrap();
    pub static ref BASS_SMOOTH_INTEGRATED_NAME: CString = CString::new("bass_smooth_integrated").unwrap();

    // mid
    pub static ref MID_NAME: CString = CString::new("mid").unwrap();
    pub static ref MID_SMOOTH_NAME: CString = CString::new("mid_smooth").unwrap();
    pub static ref MID_INTEGRATED_NAME: CString = CString::new("mid_integrated").unwrap();
    pub static ref MID_SMOOTH_INTEGRATED_NAME: CString = CString::new("mid_smooth_integrated").unwrap();

    // high
    pub static ref HIGH_NAME: CString = CString::new("high").unwrap();
    pub static ref HIGH_SMOOTH_NAME: CString = CString::new("high_smooth").unwrap();
    pub static ref HIGH_INTEGRATED_NAME: CString = CString::new("high_integrated").unwrap();
    pub static ref HIGH_SMOOTH_INTEGRATED_NAME: CString = CString::new("high_smooth_integrated").unwrap();
}

#[derive(Debug, Clone, Copy)]
pub enum Uniform {
    Float(GLfloat),
    Vec2(GLfloat, GLfloat),
    Vec3(GLfloat, GLfloat, GLfloat),
    Vec4(GLfloat, GLfloat, GLfloat, GLfloat),
    Mat2([GLfloat; 4]),
    Mat3([GLfloat; 9]),
    Mat4([GLfloat; 16]),
    Mat2x3([GLfloat; 6]),
    Mat3x2([GLfloat; 6]),
    Mat2x4([GLfloat; 8]),
    Mat4x2([GLfloat; 8]),
    Mat3x4([GLfloat; 12]),
    Mat4x3([GLfloat; 12]),
}

impl Uniform {
    pub fn from_yaml(value: &serde_yaml::Value) -> Result<Self> {
        let this = match value {
            serde_yaml::Value::Bool(b) => Self::Float(*b as u8 as _),
            serde_yaml::Value::Number(n) => Self::Float(n.as_f64().unwrap() as _),
            serde_yaml::Value::Sequence(s) => {
                let seq_len = s.len();

                if seq_len > 4 {
                    bail!("Uniform has too many components");
                }

                // handle matrix
                if s.iter().any(|v| v.is_sequence()) {
                    todo!();
                }

                let mut arr = [0_f32; 4];
                for (index, value) in s.into_iter().enumerate() {
                    match value.as_f64() {
                        Some(comp) => arr[index] = comp as _,
                        None => bail!(
                            "Expected vector component to be a number, got \"{:?}\"",
                            value
                        ),
                    }
                }

                match &arr[..seq_len] {
                    &[x] => Self::Float(x),
                    &[x, y] => Self::Vec2(x, y),
                    &[x, y, z] => Self::Vec3(x, y, z),
                    &[x, y, z, w] => Self::Vec4(x, y, z, w),
                    _ => unreachable!(),
                }
            }

            value => bail!(
                "Expected uniform to be a bool, number, vector or matrix, got \"{:?}\"",
                value
            ),
        };

        Ok(this)
    }

    pub fn bind(&self, location: GLint) {
        unsafe {
            match self {
                Uniform::Float(v0) => gl::Uniform1f(location, *v0),
                Uniform::Vec2(v0, v1) => gl::Uniform2f(location, *v0, *v1),
                Uniform::Vec3(v0, v1, v2) => gl::Uniform3f(location, *v0, *v1, *v2),
                Uniform::Vec4(v0, v1, v2, v3) => gl::Uniform4f(location, *v0, *v1, *v2, *v3),
                Uniform::Mat2(vs) => gl::UniformMatrix2fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat3(vs) => gl::UniformMatrix3fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat4(vs) => gl::UniformMatrix4fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat2x3(vs) => gl::UniformMatrix2x3fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat3x2(vs) => gl::UniformMatrix3x2fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat2x4(vs) => gl::UniformMatrix2x4fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat4x2(vs) => gl::UniformMatrix4x2fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat3x4(vs) => gl::UniformMatrix3x4fv(location, 1, gl::FALSE, vs as _),
                Uniform::Mat4x3(vs) => gl::UniformMatrix4x3fv(location, 1, gl::FALSE, vs as _),
            }
        }
    }
}
