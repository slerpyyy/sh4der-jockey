use std::{ffi::CString, mem::MaybeUninit};

use anyhow::{bail, Result};
use gl::types::*;
use lazy_static::lazy_static;
use serde_yaml::Value;

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

#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn from_yaml(value: &Value) -> Result<Self> {
        let this = match value {
            Value::Bool(b) => Self::Float(*b as u8 as _),
            Value::Number(n) => Self::Float(n.as_f64().unwrap() as _),
            Value::Sequence(seq) => {
                let seq_len = seq.len();
                if seq_len > 4 || seq_len == 0 {
                    bail!(
                        "Uniform must have between 1 and 4 components, got \"{:?}\"",
                        seq
                    );
                }

                // handle matrix
                if let Some(width) = seq
                    .iter()
                    .filter_map(Value::as_sequence)
                    .map(Vec::len)
                    .max()
                {
                    let height = seq_len;
                    let mut matrix = match (width, height) {
                        (2, 2) => Self::Mat2([0.0; 4]),
                        (3, 3) => Self::Mat3([0.0; 9]),
                        (4, 4) => Self::Mat4([0.0; 16]),
                        (2, 3) => Self::Mat2x3([0.0; 6]),
                        (3, 2) => Self::Mat3x2([0.0; 6]),
                        (2, 4) => Self::Mat2x4([0.0; 8]),
                        (4, 2) => Self::Mat4x2([0.0; 8]),
                        (3, 4) => Self::Mat3x4([0.0; 12]),
                        (4, 3) => Self::Mat4x3([0.0; 12]),
                        _ => bail!("Invalid uniform matrix format, got \"{:?}\"", seq),
                    };

                    // fill matrix
                    let slice = matrix.mat_slice_mut().unwrap();
                    for (y, row) in seq.iter().enumerate() {
                        let row = match row {
                            s @ Value::Number(_) => vec![s.clone(); width],
                            Value::Sequence(row) => row.clone(),
                            s => bail!("Matrix row must be a vector or number, got \"{:?}\"", s),
                        };

                        for (x, val) in row.into_iter().enumerate() {
                            let val = match val.as_f64() {
                                Some(s) => s as _,
                                None => bail!(
                                    "Uniform matrix component must be a number, got \"{:?}\"",
                                    val
                                ),
                            };

                            // implicitely transpose matrix
                            slice[x * height + y] = val;
                        }
                    }

                    return Ok(matrix);
                }

                let mut arr = [0_f32; 4];
                for (index, value) in seq.into_iter().enumerate() {
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

    pub fn transpose(&mut self) -> Result<(), ()> {
        fn helper<const W: usize, const N: usize>(vs: [GLfloat; N]) -> [GLfloat; N] {
            let mut out: [GLfloat; N] = unsafe { MaybeUninit::uninit().assume_init() };
            for (index, &val) in vs.iter().enumerate() {
                let y = index % W;
                let x = index / W;
                let i = x + (N / W) * y;
                out[i] = val;

                dbg!(i, &out);
            }

            out
        }

        match self {
            Uniform::Mat2(vs) => *vs = helper::<2, 4>(*vs),
            Uniform::Mat3(vs) => *vs = helper::<3, 9>(*vs),
            Uniform::Mat4(vs) => *vs = helper::<4, 16>(*vs),

            Uniform::Mat2x3(vs) => *self = Uniform::Mat3x2(helper::<3, 6>(*vs)),
            Uniform::Mat3x2(vs) => *self = Uniform::Mat2x3(helper::<2, 6>(*vs)),
            Uniform::Mat2x4(vs) => *self = Uniform::Mat4x2(helper::<4, 8>(*vs)),
            Uniform::Mat4x2(vs) => *self = Uniform::Mat2x4(helper::<2, 8>(*vs)),
            Uniform::Mat3x4(vs) => *self = Uniform::Mat4x3(helper::<4, 12>(*vs)),
            Uniform::Mat4x3(vs) => *self = Uniform::Mat3x4(helper::<3, 12>(*vs)),

            _ => return Err(()),
        };

        Ok(())
    }

    fn mat_slice_mut(&mut self) -> Option<&mut [GLfloat]> {
        match self {
            Uniform::Mat2(vs) => Some(vs),
            Uniform::Mat3(vs) => Some(vs),
            Uniform::Mat4(vs) => Some(vs),
            Uniform::Mat2x3(vs) => Some(vs),
            Uniform::Mat3x2(vs) => Some(vs),
            Uniform::Mat2x4(vs) => Some(vs),
            Uniform::Mat4x2(vs) => Some(vs),
            Uniform::Mat3x4(vs) => Some(vs),
            Uniform::Mat4x3(vs) => Some(vs),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_float() {
        let value = serde_yaml::from_str("2.3").unwrap();
        let uniform = Uniform::from_yaml(&value).unwrap();

        assert_eq!(uniform, Uniform::Float(2.3));
    }

    #[test]
    fn parse_vec_simple() {
        let value = serde_yaml::from_str("[1, 2, 3]").unwrap();
        let uniform = Uniform::from_yaml(&value).unwrap();

        assert_eq!(uniform, Uniform::Vec3(1.0, 2.0, 3.0));
    }

    #[test]
    fn parse_vec_mixed() {
        let value = serde_yaml::from_str("[2.3, -5, 0, 7]").unwrap();
        let uniform = Uniform::from_yaml(&value).unwrap();

        assert_eq!(uniform, Uniform::Vec4(2.3, -5.0, 0.0, 7.0));
    }

    #[test]
    fn parse_matrix_simple() {
        let value = serde_yaml::from_str("[[1, 2], [3, 4]]").unwrap();
        let mut uniform = Uniform::from_yaml(&value).unwrap();

        assert_eq!(uniform, Uniform::Mat2([1.0, 3.0, 2.0, 4.0]));

        uniform.transpose().unwrap();

        assert_eq!(uniform, Uniform::Mat2([1.0, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn parse_matrix_chaotic() {
        let value = serde_yaml::from_str("[[1, 2], 5.2, [], [0, 0, -4]]").unwrap();
        let mut uniform = Uniform::from_yaml(&value).unwrap();

        #[rustfmt::skip]
        assert_eq!(uniform, Uniform::Mat3x4([
            1.0, 5.2, 0.0, 0.0,
            2.0, 5.2, 0.0, 0.0,
            0.0, 5.2, 0.0, -4.0,
        ]));

        uniform.transpose().unwrap();

        #[rustfmt::skip]
        assert_eq!(uniform, Uniform::Mat4x3([
            1.0, 2.0, 0.0,
            5.2, 5.2, 5.2,
            0.0, 0.0, 0.0,
            0.0, 0.0, -4.0
        ]));
    }
}
