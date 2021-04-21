use std::collections::HashMap;

use crate::texture::*;
use crate::util::*;
use gl::types::*;
use serde_json::Value;

const DEFAULT_VERTEX_SHADER: &str = include_str!("defaults/vs.glsl");
const DEFAULT_FRAGMENT_SHADER: &str = include_str!("defaults/fs.glsl");

#[derive(Debug)]
pub struct Stage {
    pub prog_id: GLuint,
    pub target: Option<String>,
    pub vs_id: Option<GLuint>,
    pub fs_id: Option<GLuint>,
    pub perf: RunningAverage,
}

impl Stage {
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

        let target = match object.get("target") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => panic!("expected string, got {:?}", s),
        };

        let vs_id = compile_shader(&vs, gl::VERTEX_SHADER);
        let fs_id = compile_shader(&fs, gl::FRAGMENT_SHADER);
        let prog_id = link_program(vs_id, fs_id);

        let perf = RunningAverage::new();

        Some(Stage {
            prog_id,
            target,
            vs_id: Some(vs_id),
            fs_id: Some(fs_id),
            perf,
        })
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        unsafe {
            if let Some(id) = self.vs_id {
                gl::DetachShader(self.prog_id, id);
                gl::DeleteShader(id);
            }

            if let Some(id) = self.fs_id {
                gl::DetachShader(self.prog_id, id);
                gl::DeleteShader(id);
            }

            gl::DeleteProgram(self.prog_id);
        }
    }
}

#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<String, Texture>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new(), buffers: HashMap::new() }
    }

    pub fn from_json(object: Value) -> Option<Self> {
        let passes = match object.get("stages") {
            Some(serde_json::Value::Array(s)) => s,
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

            let texture = Texture::new(1080, 720, k as _);
            buffers.insert(target.clone(), texture);
        }

        Some(Self { stages, buffers })
    }
}
