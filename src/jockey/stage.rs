use crate::util::*;
use gl::types::*;
use serde_json::Value;
use std::ffi::CString;

const PASS_VERT: &str = include_str!("shaders/pass.vert");
const PASS_FRAG: &str = include_str!("shaders/pass.frag");

#[derive(Debug)]
pub enum StageKind {
    Comp {
        tex_type: GLuint,
        tex_dim: [u32; 3],
    },
    Vert {
        count: GLsizei,
        mode: GLenum,
        res: Option<(u32, u32)>,
    },
    Frag {
        res: Option<(u32, u32)>,
    },
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
}

impl Stage {
    pub fn from_json(object: Value) -> Result<Self, String> {
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

        let res = match object.get("res").or_else(|| object.get("resolution")) {
            Some(Value::Array(arr)) if arr.len() == 2 => {
                let err_msg = "Resolution not a positive integer";
                Some((
                    arr[0].as_u64().expect(err_msg) as _,
                    arr[1].as_u64().expect(err_msg) as _,
                ))
            }
            _ => None,
        };

        // read all shaders to strings
        let shaders: [Option<String>; 3] = {
            let mut out = [None, None, None];
            for (k, &name) in ["vs", "fs", "cs"].iter().enumerate() {
                out[k] = match object.get(name) {
                    Some(Value::String(s)) => match std::fs::read_to_string(s) {
                        Ok(s) => Some(s),
                        Err(e) => return Err(e.to_string()),
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
                let fs = preprocess(&fs)?;

                let vs_id = compile_shader(&vs, gl::VERTEX_SHADER)?;
                let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER)?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let kind = StageKind::Frag { res };

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                })
            }

            // handle vertex shader stages
            [Some(vs), fs_opt, None] => {
                let fs = fs_opt.unwrap_or_else(|| PASS_FRAG.to_string());
                let vs = preprocess(&vs)?;
                let fs = preprocess(&fs)?;

                let vs_id = compile_shader(&vs, gl::VERTEX_SHADER)?;
                let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER)?;

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

                let kind = StageKind::Vert { res, count, mode };

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                })
            }

            // handle compute shader stages
            [None, None, Some(cs)] => {
                let cs = preprocess(&cs)?;

                let tex_type = match object.get("cs_type") {
                    Some(Value::String(s)) if s.as_str() == "1D" => 1,
                    Some(Value::String(s)) if s.as_str() == "2D" => 2,
                    Some(Value::String(s)) if s.as_str() == "3D" => 3,
                    s => return Err(format!("Expected texture type, got {:?}", s)),
                };

                let tex_dim = match object.get("cs_size") {
                    Some(Value::Array(ar)) if ar.len() <= 3 => {
                        let mut tex_dim: [u32; 3] = [0; 3];
                        for (i, sz) in ar.iter().enumerate() {
                            let val = sz.as_u64();
                            tex_dim[i] = match val {
                                Some(dim) => dim as _,
                                _ => return Err(format!("Texture size not an integer: {:?}", val)),
                            };
                        }
                        tex_dim
                    }

                    Some(Value::Number(n)) => [
                        match n.as_u64() {
                            Some(k) => k as _,
                            _ => return Err(format!("Texture size not an integer: {:?}", n)),
                        },
                        0,
                        0,
                    ],

                    s => return Err(format!("Expected texture size, got {:?}", s)),
                };

                let cs_id = compile_shader(&cs, gl::COMPUTE_SHADER)?;
                let sh_ids = vec![cs_id];
                let prog_id = link_program(&sh_ids)?;

                let kind = StageKind::Comp { tex_type, tex_dim };

                Ok(Stage {
                    prog_id,
                    target,
                    kind,
                    sh_ids,
                    deps,
                    perf,
                })
            }

            // Handle everything else
            _ => Err("Invalid shader configuration".to_string()),
        }
    }

    pub fn resolution(&self) -> Option<[u32; 3]> {
        match self.kind {
            StageKind::Comp { tex_dim, .. } => Some(tex_dim),
            StageKind::Frag {
                res: Some((width, height)),
                ..
            }
            | StageKind::Vert {
                res: Some((width, height)),
                ..
            } => Some([width, height, 0]),
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
