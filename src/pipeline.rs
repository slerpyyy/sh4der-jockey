use std::collections::HashMap;

use crate::util::*;
use gl::types::*;
use serde_json::Value;

const DEFAULT_VERTEX_SHADER: &str = include_str!("defaults/vs.glsl");
const DEFAULT_FRAGMENT_SHADER: &str = include_str!("defaults/fs.glsl");

#[derive(Debug, Clone)]
pub struct Stage {
    pub prog_id: GLuint,
    pub target: Option<String>,
}

impl Stage {
    pub fn new(prog_id: GLuint, target: Option<String>) -> Self {
        Self { prog_id, target }
    }

    pub fn from_json(object: Value) -> Option<Self> {
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

        let target = match object.get("TARGET") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => panic!("expected string, got {:?}", s),
        };

        let vs = compile_shader(&vs, gl::VERTEX_SHADER);
        let fs = compile_shader(&fs, gl::FRAGMENT_SHADER);

        let prog_id = link_program(vs, fs);

        Some(Stage::new(prog_id, target))
    }
}

#[derive(Debug, Clone)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<String, GLuint>,
}

impl Pipeline {
    pub fn from_json(object: Value) -> Option<Self> {
        let passes = match object.get("PASSES") {
            Some(serde_json::Value::Array(s)) => s,
            s => panic!("expected array, got {:?}", s),
        }
        .clone();

        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_json(pass)?;
            stages.push(stage);
        }

        let mut buffers = HashMap::new();
        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            if buffers.contains_key(target) {
                continue;
            }

            let mut tex_id = 0;
            unsafe {
                gl::GenFramebuffers(1, &mut tex_id);
            }
            buffers.insert(target.clone(), tex_id);
        }

        Some(Self { stages, buffers })
    }
}
