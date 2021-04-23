use crate::util::*;
use gl::types::*;
use serde_json::Value;

const PASS_VERT: &str = include_str!("shaders/pass.vert");
const PASS_FRAG: &str = include_str!("shaders/pass.frag");

#[derive(Debug)]
pub enum StageKind {
    Comp { tex_type: GLuint, tex_dim: [u32; 3] },
    Vert { count: GLsizei, mode: GLenum },
    Frag,
}

/// The stage struct
///
/// This struct holds all data accosiated to a stage in the render pipeline.
///
/// Note that it does not render anything itself, it merely holds the
/// information and takes care of resource management, i.e. it compiles
/// all shaders and links all programs on initialization and makes sure all
/// shaders and programs are deleted once they're no longer needed.
#[derive(Debug)]
pub struct Stage {
    pub prog_id: GLuint,
    pub target: Option<String>,
    pub kind: StageKind,
    pub sh_ids: Vec<GLuint>,
    pub perf: RunningAverage<f32, 128>,
}

impl Stage {
    pub fn from_json(object: Value) -> Result<Self, String> {
        let perf = RunningAverage::new();

        // get render target name
        let target = match object.get("target") {
            Some(Value::String(s)) => Some(s.clone()),
            Some(s) => {
                return Err(format!(
                    "Expected field \"target\" to be a string, got {:?}",
                    s
                ))
            }
            None => None,
        };

        // read all shaders to strings
        let shaders: [Option<String>; 3] = {
            let mut out = [None, None, None];
            for (k, name) in ["vs", "fs", "cs"].iter().enumerate() {
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

                let vs_id = compile_shader(&vs, gl::VERTEX_SHADER)?;
                let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER)?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let kind = StageKind::Frag {};

                Ok(Stage {
                    prog_id,
                    target,
                    sh_ids,
                    perf,
                    kind,
                })
            }

            // handle vertex shader stages
            [Some(vs), fs_opt, None] => {
                let fs = fs_opt.unwrap_or_else(|| PASS_FRAG.to_string());

                let vs_id = compile_shader(&vs, gl::VERTEX_SHADER)?;
                let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER)?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let kind = StageKind::Vert {
                    count: 1000,
                    mode: gl::LINES,
                };

                Ok(Stage {
                    prog_id,
                    target,
                    sh_ids,
                    perf,
                    kind,
                })
            }

            // handle compute shader stages
            [None, None, Some(cs)] => {
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
                    perf,
                    sh_ids,
                    kind,
                })
            }

            // Handle everything else
            _ => Err("Invalid shader configuration".to_string()),
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
