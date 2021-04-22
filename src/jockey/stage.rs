use gl::types::*;
use crate::util::*;
use serde_json::Value;

const DEFAULT_VERTEX_SHADER: &str = include_str!("../defaults/vs.glsl");
const DEFAULT_FRAGMENT_SHADER: &str = include_str!("../defaults/fs.glsl");

#[derive(Debug)]
pub enum StageKind {
    Comp {
        tex_type: GLuint,
        tex_dim: [u32; 3],
    },
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

        let target = match object.get("target") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => return Err(format!("expected string, got {:?}", s)),
        };

        let cs = match object.get("cs") {
            Some(Value::String(s)) => {
                match std::fs::read_to_string(s) {
                    Ok(s) => Some(s),
                    Err(e) => return Err(e.to_string()),
                }
            }
            None => None,
            s => return Err(format!("expected string, got {:?}", s)),
        };

        if let Some(cs) = cs {
            let tex_type = match object.get("cs_type") {
                Some(Value::String(s)) if s.as_str() == "1D" => gl::TEXTURE_1D,
                Some(Value::String(s)) if s.as_str() == "2D" => gl::TEXTURE_2D,
                Some(Value::String(s)) if s.as_str() == "3D" => gl::TEXTURE_3D,
                s => return Err(format!("expected texture type, got {:?}", s)),
            };

            let tex_dim = match object.get("cs_size") {
                Some(Value::Array(ar)) if ar.len() <= 3 => {
                    let mut tex_dim: [u32; 3] = [0; 3];
                    for (i, sz) in ar.iter().enumerate() {
                        let val = sz.as_u64();
                        tex_dim[i] = match val {
                            Some(dim) => dim as _,
                            _ => return Err(format!("texture size not an integer: {:?}", val)),
                        };
                    }
                    tex_dim
                }

                Some(Value::Number(n)) => [
                    match n.as_u64() {
                        Some(k) => k as _,
                        _ => return Err(format!("texture size not an integer: {:?}", n)),
                    },
                    0,
                    0,
                ],

                s => return Err(format!("expected texture size, got {:?}", s)),
            };

            let cs_id = compile_shader(&cs, gl::COMPUTE_SHADER)?;
            let sh_ids = vec![cs_id];
            let prog_id = link_program(&sh_ids)?;

            let kind = StageKind::Comp {
                tex_type,
                tex_dim,
            };

            Ok(Stage {
                prog_id,
                target,
                perf,
                sh_ids,
                kind,
            })
        } else {
            let fs = match object.get("fs") {
                Some(Value::String(s)) => {
                    match std::fs::read_to_string(s) {
                        Ok(s) => s,
                        Err(e) => return Err(e.to_string()),
                    }
                }
                None => DEFAULT_FRAGMENT_SHADER.to_string(),
                s => return Err(format!("expected string, got {:?}", s)),
            };

            let vs = match object.get("vs") {
                Some(Value::String(s)) => {
                    match std::fs::read_to_string(s) {
                        Ok(s) => s,
                        Err(e) => return Err(e.to_string()),
                    }
                }
                None => DEFAULT_VERTEX_SHADER.to_string(),
                s => return Err(format!("expected string, got {:?}", s)),
            };

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
