use crate::jockey::*;
use serde_yaml::Value;
use std::{collections::HashMap, ffi::CString};

/// The rendering pipeline struct
///
/// This struct holds the structure of the rendering pipeline. Note that it
/// does not render anything itself, it merely holds the information and takes
/// care of resource management.
#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<CString, Box<dyn Texture>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            buffers: HashMap::new(),
        }
    }

    pub fn load(window: &sdl2::video::Window) -> Result<Self, String> {
        let reader = match std::fs::File::open("pipeline.yaml") {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let object = match serde_yaml::from_reader(reader) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let screen_size = window.size();

        Pipeline::from_yaml(object, screen_size)
    }

    pub fn from_yaml(object: Value, screen_size: (u32, u32)) -> Result<Self, String> {
        let passes = match object.get("stages") {
            Some(Value::Sequence(s)) => s.clone(),
            None => return Err("Required field \"stages\" not found".to_string()),
            s => return Err(format!("Expected \"stages\" to be an array, got {:?}", s)),
        };

        let fft_window = match object.get("fftWindowSize") {
            Some(Value::Number(n)) => {
                let n = n.as_u64().unwrap();
                if n.count_ones() != 1 {
                    return Err(format!(
                        "Expected fftWindow to be a power of 2, got: {:?}",
                        n
                    ));
                }
                n
            }
            None => 8192_u64,
            s => return Err(format!("Expected fftWindow to be number, got: {:?}", s)),
        };

        // parse stages
        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_yaml(pass)?;
            stages.push(stage);
        }

        // put buffers into hashmap
        let mut buffers = HashMap::<CString, Box<dyn Texture>>::new();

        let audio_samples_texture = TextureStruct::texture_from_params(
            &[fft_window as _],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );
        buffers.insert(
            CString::new("samples").unwrap(),
            Box::new(audio_samples_texture),
        );

        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            // check if target exists already
            if let Some(tex) = buffers.get(target) {
                if Some(tex.resolution()) != stage.resolution() {
                    return Err(format!(
                        "Texture {:?} already has a different resolution",
                        target
                    ));
                }

                continue;
            }

            // create textures
            let texture: Box<dyn Texture> = match stage.kind {
                StageKind::Frag { res } | StageKind::Vert { res, .. } => {
                    let (width, height) = res.unwrap_or(screen_size);
                    Box::new(FrameBuffer::new(width as _, height as _))
                }
                StageKind::Comp {
                    tex_type, tex_dim, ..
                } => Box::new(TextureStruct::new(&tex_dim[..(tex_type as _)])),
            };

            // insert texture into hashmap
            buffers.insert(target.clone(), texture);
        }

        // compute uniform dependencies
        for stage in stages.iter_mut() {
            for tex_name in buffers.keys() {
                // try to locate the uniform in the program
                let loc = unsafe { gl::GetUniformLocation(stage.prog_id, tex_name.as_ptr()) };

                // add uniform to list of dependencies
                if loc != -1 {
                    stage.deps.push(tex_name.clone());
                }
            }
        }

        Ok(Self { stages, buffers })
    }

    pub fn resize_buffers(&mut self, width: u32, height: u32) {
        for stage in self.stages.iter() {
            match &stage.target {
                Some(s) => {
                    let tex = match stage.kind {
                        StageKind::Frag { res: None, .. } | StageKind::Vert { res: None, .. } => {
                            FrameBuffer::new(width, height)
                        }
                        _ => continue,
                    };
                    self.buffers.insert(s.clone(), Box::new(tex));
                }
                None => continue,
            }
        }
    }
}
