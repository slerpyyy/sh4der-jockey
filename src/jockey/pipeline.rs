use std::collections::HashMap;
use crate::jockey::*;
use serde_json::Value;

/// The rendering pipeline struct
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
        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            if buffers.contains_key(target) {
                continue;
            }
            let texture = match stage.kind {
                StageKind::Frag { .. } => Texture::new(1280, 720),
                StageKind::Comp {
                    tex_type, tex_dim, ..
                } => Texture::create_image_texture(tex_type, tex_dim),
            };
            buffers.insert(target.clone(), texture);
        }

        Some(Self { stages, buffers })
    }
}
