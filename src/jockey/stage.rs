use std::ffi::CString;

use gl::types::*;
use serde_yaml::Value;

use super::geometry_from_gltf::*;

use crate::{jockey::Geometry, util::*};

pub const PASS_VERT: &str = include_str!("shaders/pass.vert");
pub const PASS_FRAG: &str = include_str!("shaders/pass.frag");

#[derive(Debug)]
pub enum StageKind {
    Comp {
        dispatch: [GLuint; 3],
    },
    Vert {
        count: GLsizei,
        mode: GLenum,
        geometries: Option<Vec<Geometry>>,
        thickness: f32,
    },
    Frag {},
}

/// The stage struct
///
/// This struct holds all data associated to a stage in the render pipeline.
///
/// Note that it does not render anything itself, it merely holds the
/// information and takes care of resource management, i.e. it compiles
/// all shaders and links all programs on initialization and makes sure all
/// shaders and programs are deleted once they're no longer needed.
#[derive(Debug)]
pub struct Stage {
    pub prog_id: GLuint,
    pub target: Option<CString>,
    pub kind: StageKind,
    pub sh_ids: Vec<GLuint>,
    pub deps: Vec<CString>,
    pub perf: RunningAverage<f32, 128>,
    pub builder: TextureBuilder,
}

impl Stage {
    pub fn from_yaml(object: Value) -> Result<Self, String> {
        let perf = RunningAverage::new();
        let deps = Vec::new();

        // get render target name
        let target = match object.get("target") {
            Some(Value::String(s)) => Some(CString::new(s.as_str()).unwrap()),
            Some(s) => {
                return Err(format!(
                    "Expected field \"target\" to be a string, got {:?}",
                    s
                ))
            }
            None => None,
        };

        // read all shaders to strings
        let mut lut = Vec::new();
        let shaders: [Option<(String, String)>; 3] = {
            let mut out = [None, None, None];
            for (k, &name) in ["vs", "fs", "cs"].iter().enumerate() {
                out[k] = match object.get(name) {
                    Some(Value::String(f)) => match std::fs::read_to_string(f) {
                        Ok(s) => Some((s, f.into())),
                        Err(e) => return Err(format!("{}, {}", e.to_string(), f)),
                    },
                    Some(s) => {
                        return Err(format!(
                            "Expected shader field to be a filename, got {:?}",
                            s
                        ))
                    }
                    None => None,
                }
            }

            out
        };

        match shaders {
            // handle full screen fragment shader stages
            [None, Some(fs), None] => {
                let vs = PASS_VERT;
                let fs = preprocess(&fs.0, &fs.1, &mut lut)?;

                let vs_id =
                    compile_shader(&vs, gl::VERTEX_SHADER).map_err(|e| process_error(e, &lut))?;
                let fs_id =
                    compile_shader(&fs, gl::FRAGMENT_SHADER).map_err(|e| process_error(e, &lut))?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let builder = TextureBuilder::parse(&object, true, true)?;

                if !matches!(builder.resolution.as_slice(), &[] | &[_, _]) {
                    return Err("Expected \"resolution\" to be 2D".into());
                }

                let kind = StageKind::Frag {};

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                    builder,
                })
            }

            // handle vertex shader stages
            [Some(vs), fs_opt, None] => {
                let vs = preprocess(&vs.0, &vs.1, &mut lut)?;
                let fs = match fs_opt {
                    Some(fs) => preprocess(&fs.0, &fs.1, &mut lut)?,
                    None => PASS_FRAG.into(),
                };

                let vs_id =
                    compile_shader(&vs, gl::VERTEX_SHADER).map_err(|e| process_error(e, &lut))?;
                let fs_id =
                    compile_shader(&fs, gl::FRAGMENT_SHADER).map_err(|e| process_error(e, &lut))?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let count = match object.get("count") {
                    Some(s) => match s.as_u64() {
                        Some(n) => n as _,
                        _ => {
                            return Err(format!(
                                "Expected vertex count to be an unsigned int, got {:?}",
                                s
                            ))
                        }
                    },
                    _ => 1024,
                };

                let mode = match object.get("mode") {
                    Some(s) => match s.as_str() {
                        Some("LINE_LOOP") => gl::LINE_LOOP,
                        Some("LINE_STRIP") => gl::LINE_STRIP,
                        Some("LINES") => gl::LINES,
                        Some("POINTS") => gl::POINTS,
                        Some("TRIANGLE_FAN") => gl::TRIANGLE_FAN,
                        Some("TRIANGLE_STRIP") => gl::TRIANGLE_STRIP,
                        Some("TRIANGLES") => gl::TRIANGLES,
                        _ => return Err(format!("Invalid vertex mode: {:?}", s)),
                    },
                    _ => gl::TRIANGLES,
                };

                let thickness = match object
                    .get("thickness")
                    .or(object.get("stroke_weight"))
                    .or(object.get("point_size"))
                    .or(object.get("line_width"))
                    .map(Value::as_f64)
                {
                    Some(Some(t)) if t > 0.0 => t as f32,
                    None => 1.0,
                    Some(s) => {
                        return Err(format!(
                            "Expected \"thickness\" to be positive float, got {:?}",
                            s
                        ))
                    }
                };

                let gltf = match object.get("gltf") {
                    Some(s) => match s.as_str() {
                        Some(value) => {
                            match geometry_from_gltf(value.to_string()) {
                                Ok(s) => Some(s),
                                Err(e) => {
                                    log::warn!("{}", e);
                                    None
                                },
                            }
                        },
                        _ => {
                            log::warn!("Invalid path detected: {:?}", s);
                            None
                        },
                    },
                    _ => None,
                };

                let builder = TextureBuilder::parse(&object, true, true)?;

                if !matches!(builder.resolution.as_slice(), &[] | &[_, _]) {
                    return Err("Expected \"resolution\" to be 2D".into());
                }

                let kind = StageKind::Vert {
                    count,
                    mode,
                    geometries: gltf,
                    thickness,
                };

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                    builder,
                })
            }

            // handle compute shader stages
            [None, None, Some(cs)] => {
                let cs = preprocess(&cs.0, &cs.1, &mut lut)?;

                let cs_id =
                    compile_shader(&cs, gl::COMPUTE_SHADER).map_err(|e| process_error(e, &lut))?;
                let sh_ids = vec![cs_id];
                let prog_id = link_program(&sh_ids)?;

                // get target resolution
                let dispatch = match object
                    .get("dispatch_size")
                    .or_else(|| object.get("dispatch"))
                {
                    Some(Value::Sequence(dims)) => {
                        if dims.is_empty() || dims.len() > 3 {
                            return Err(format!(
                                "Field \"dispatch_size\" must be a list of 1 to 3 numbers, got {} elements",
                                dims.len()
                            ));
                        }

                        let mut out = [1; 3];
                        for (k, dim) in dims.iter().enumerate() {
                            match dim.as_i64() {
                                Some(n) if n > 0 => {
                                    // OpenGL must allow 65535 in each dimension
                                    // https://www.khronos.org/opengl/wiki/Compute_Shader#Limitations
                                    if n > 65535 {
                                        return Err(format!(
                                            "Values of \"dispatch_size\" may not exceed 65535 in any dimension, got {:?}",
                                            n
                                        ))
                                    }

                                    out[k] = n as _
                                },
                                _ => return Err(format!(
                                    "Expected \"dispatch_size\" to be a list of positive numbers, got {:?}",
                                    dims
                                )),
                            };
                        }

                        out
                    }
                    Some(s) => {
                        return Err(format!(
                        "Expected \"dispatch_size\" to be a list of unsigned integers, got {:?}",
                        s
                    ))
                    }
                    None => {
                        return Err(
                            "Field \"dispatch_size\" is mandatory for compute shaders".into()
                        )
                    }
                };

                let builder = TextureBuilder::parse(&object, true, false)?;

                if builder.resolution.as_slice().is_empty() {
                    return Err("Field \"resolution\" is mandatory for compute shaders".into());
                }

                if target.is_none() {
                    return Err("Field \"target\" is mandatory for compute shaders".into());
                }

                let kind = StageKind::Comp { dispatch };

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                    builder,
                })
            }

            // Handle everything else
            _ => Err("Invalid shader configuration".to_string()),
        }
    }

    pub fn resolution(&self) -> Option<[u32; 3]> {
        match self.builder.resolution.as_slice() {
            &[w] => Some([w, 0, 0]),
            &[w, h] => Some([w, h, 0]),
            &[w, h, d] => Some([w, h, d]),
            _ => None,
        }
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        unsafe {
            for &id in self.sh_ids.iter() {
                gl::DetachShader(self.prog_id, id);
                gl::DeleteShader(id);
            }

            gl::DeleteProgram(self.prog_id);
        }
    }
}
