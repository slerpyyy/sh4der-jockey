use std::collections::HashMap;

use crate::util::*;
use gl::types::*;
use serde_json::Value;

const DEFAULT_VERTEX_SHADER: &str = include_str!("defaults/vs.glsl");
const DEFAULT_FRAGMENT_SHADER: &str = include_str!("defaults/fs.glsl");

/// The rendering pipeline
///
/// This struct holds the structure of the rendering pipeline. Note that it
/// does not render anything itself, it merely holds the information and takes
/// care of resource management, i.e. it compiles all shaders and links all
/// programs on initialization and makes sure all shaders and programs are
/// deleted once they're no longer needed.
#[derive(Debug)]
pub struct ComputeStage {
    pub tex_type: GLuint,
    pub tex_dim: [u32; 3],
}

#[derive(Debug)]
pub struct RegularStage {
    pub vs_id: Option<GLuint>,
    pub fs_id: Option<GLuint>,
}

#[derive(Debug)]
enum StageKind {
    Comp(ComputeStage),
    Frag(RegularStage),
}

#[derive(Debug)]
pub struct Stage {
    pub prog_id: GLuint,
    pub target: Option<String>,
    pub perf: RunningAverage<f32, 128>,
    pub kind: StageKind,
}

impl Stage {
    pub fn from_json(object: Value) -> Option<Self> {
        let target = match object.get("target") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => panic!("expected string, got {:?}", s),
        };
        let perf = RunningAverage::new();

        let cs = match object.get("cs") {
            Some(Value::String(s)) => {
                Some(std::fs::read_to_string(s).expect("could not read file"))
            }
            None => None,
            s => panic!("expected string, got {:?}", s),
        };

        if cs != None {
            let tex_type = match object.get("cs_type") {
                Some(Value::String(s)) if s.as_str() == "1D" => gl::TEXTURE_1D,
                Some(Value::String(s)) if s.as_str() == "2D" => gl::TEXTURE_2D,
                Some(Value::String(s)) if s.as_str() == "3D" => gl::TEXTURE_3D,
                s => panic!("expected texture type, got {:?}", s),
            };

            let tex_dim = match object.get("cs_size") {
                Some(Value::Array(ar)) if ar.len() <= 3 => {
                    let tex_dim: [u32; 3] = [0; 3];
                    for (i, sz) in ar.iter().enumerate() {
                        let val = sz.as_u64();
                        tex_dim[i] =
                            val.expect(&format!("texture size not an integer: {:?}", val)) as _;
                    }
                    tex_dim
                }

                Some(Value::Number(n)) => [
                    n.as_u64()
                        .expect(&format!("texture size not an integer: {:?}", n))
                        as _,
                    0,
                    0,
                ],
                s => panic!("expected texture size, got {:?}", s),
            };

            let cs_id = compile_shader(&cs.unwrap(), gl::COMPUTE_SHADER);
            let kind = StageKind::Comp(ComputeStage { tex_type, tex_dim });

            return Some(Stage {
                prog_id: cs_id,
                target,
                perf,
                kind,
            });
        };

        let fs = match object.get("fs") {
            Some(Value::String(s)) => std::fs::read_to_string(s).expect("could not read file"),
            None => DEFAULT_FRAGMENT_SHADER.to_string(),
            s => panic!("expected string, got {:?}", s),
        };

        let vs = match object.get("vs") {
            Some(Value::String(s)) => std::fs::read_to_string(s).expect("could not read file"),
            None => DEFAULT_VERTEX_SHADER.to_string(),
            s => panic!("expected string, got {:?}", s),
        };

        let vs_id = compile_shader(&vs, gl::VERTEX_SHADER);
        let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER);
        let prog_id = link_program(vs_id, fs_id);

        let kind = StageKind::Frag(RegularStage {
            vs_id: Some(vs_id),
            fs_id: Some(fs_id),
        });

        Some(Stage {
            prog_id,
            target,
            perf,
            kind,
        })
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        unsafe {
            match self.kind {
                StageKind::Frag(RegularStage { vs_id, fs_id }) => {
                    if let Some(id) = vs_id {
                        gl::DetachShader(self.prog_id, id);
                        gl::DeleteShader(id);
                    }

                    if let Some(id) = fs_id {
                        gl::DetachShader(self.prog_id, id);
                        gl::DeleteShader(id);
                    }
                }
                _ => (),
            }

            gl::DeleteProgram(self.prog_id);
        }
    }
}

/// The rendering pipeline
///
/// This struct holds the structure of the rendering pipeline. Note that it
/// does not render anything itself, it merely holds the information and takes
/// care of resource management.
#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<String, Texture>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn from_json(object: Value) -> Option<Self> {
        let passes = match object.get("stages") {
            Some(Value::Array(s)) => s,
            s => panic!("expected array, got {:?}", s),
        }
        .clone();

        // parse stages
        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_json(pass)?;
            stages.push(stage);
        }

        // put buffers into hashmap
        let mut buffers = HashMap::new();
        for (k, stage) in stages.iter().enumerate() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            if buffers.contains_key(target) {
                continue;
            }
            let texture = match stage.kind {
                StageKind::Frag(_) => Texture::new(1080, 720, k as _),
                StageKind::Comp(_) => {}
            };
            buffers.insert(target.clone(), texture);
        }

        Some(Self { stages, buffers })
    }
}
