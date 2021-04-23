use crate::jockey::*;
use serde_json::Value;
use std::collections::HashMap;

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

    pub fn load(window: &sdl2::video::Window) -> Result<Self, String> {
        let reader = match std::fs::File::open("pipeline.json") {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let object = match serde_json::from_reader(reader) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let screen_size = window.size();

        Pipeline::from_json(object, screen_size)
    }

    pub fn from_json(object: Value, screen_size: (u32, u32)) -> Result<Self, String> {
        let passes = match object.get("stages") {
            Some(Value::Array(s)) => s.clone(),
            s => return Err(format!("expected array, got {:?}", s)),
        };

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
                StageKind::Frag { resolution } | StageKind::Vert { resolution, .. } => {
                    let (width, height) = resolution.unwrap_or(screen_size);
                    Texture::with_framebuffer(width as _, height as _)
                }
                StageKind::Comp {
                    tex_type, tex_dim, ..
                } => Texture::create_image_texture(tex_type, tex_dim),
            };

            buffers.insert(target.clone(), texture);
        }

        Ok(Self { stages, buffers })
    }

    pub fn resize_buffers(&mut self, width: u32, height: u32) {
        for stage in &self.stages {
            let target = &stage.target;
            match target {
                Some(s) => {
                    let tex = match stage.kind {
                        StageKind::Frag {
                            resolution: None, ..
                        }
                        | StageKind::Vert {
                            resolution: None, ..
                        } => Texture::with_framebuffer(width, height),
                        _ => continue,
                    };
                    self.buffers.insert(s.clone(), tex);
                }
                None => continue,
            }
        }
    }
}
