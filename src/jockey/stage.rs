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
    Vert {
        count: GLsizei,
        mode: GLenum,
        resolution: Option<(u32, u32)>,
    },
    Frag {
        resolution: Option<(u32, u32)>,
    },
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
            Some(s) => {
                return Err(format!(
                    "Expected field \"target\" to be a string, got {:?}",
                    s
                ))
            }
            None => None,
            s => return Err(format!("expected string, got {:?}", s)),
        };

        let resolution = match object.get("resolution") {
            Some(Value::Array(ar)) if ar.len() == 2 => {
                let err_msg = "resolution not a positive integer";
                Some((
                    ar[0].as_u64().expect(err_msg) as _,
                    ar[0].as_u64().expect(err_msg) as _,
                ))
            }
            _ => None,
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
            None => None,
            s => return Err(format!("expected string, got {:?}", s)),
        };

        match shaders {
            // handle full screen fragment shader stages
            [None, Some(fs), None] => {
                let vs = PASS_VERT;

                let vs_id = compile_shader(&vs, gl::VERTEX_SHADER)?;
                let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER)?;

                let sh_ids = vec![vs_id, fs_id];
                let prog_id = link_program(&sh_ids)?;

                let kind = StageKind::Frag { resolution };

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
                    resolution,
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
