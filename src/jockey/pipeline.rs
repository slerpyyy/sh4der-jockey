use crate::jockey::*;
use image::io::Reader as ImageReader;
use serde_yaml::Value;
use std::{collections::HashMap, ffi::CString, path::Path};

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

    pub fn load(path: impl AsRef<Path>, screen_size: (u32, u32)) -> Result<Self, String> {
        let reader = match std::fs::File::open(path) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let object = match serde_yaml::from_reader(reader) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        Pipeline::from_yaml(object, screen_size)
    }

    pub fn from_yaml(object: Value, screen_size: (u32, u32)) -> Result<Self, String> {
        let passes = match object.get("stages") {
            Some(Value::Sequence(s)) => s.clone(),
            None => return Err("Required field \"stages\" not found".to_string()),
            s => return Err(format!("Expected \"stages\" to be an array, got {:?}", s)),
        };

        // get fft texture size
        let fft_size = match object.get("fft_size") {
            None => 8192,
            Some(Value::Number(n)) => {
                let n = n.as_u64().unwrap();
                if n.is_power_of_two() {
                    return Err(format!(
                        "Expected \"fft_size\" to be a power of 2, got: {:?}",
                        n
                    ));
                }
                n
            }
            s => return Err(format!("Expected \"fft_size\" to be number, got: {:?}", s)),
        };

        // parse stages
        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_yaml(pass)?;
            stages.push(stage);
        }

        // put buffers into hashmap
        let mut buffers = HashMap::<CString, Box<dyn Texture>>::new();

        // add audio samples to buffers
        let audio_samples_texture = Texture1D::with_params(
            [fft_size as _],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        let raw_spectrums_texture = Texture1D::with_params(
            [(fft_size / 2) as _],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        let spectrums_texture = Texture1D::with_params(
            [100],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        buffers.insert(
            CString::new("samples").unwrap(),
            Box::new(audio_samples_texture),
        );

        buffers.insert(
            CString::new("raw_spectrum").unwrap(),
            Box::new(raw_spectrums_texture),
        );

        buffers.insert(
            CString::new("spectrum").unwrap(),
            Box::new(spectrums_texture),
        );

        // add noise texture
        let noise = Box::new(make_noise());
        buffers.insert(CString::new("noise").unwrap(), noise);

        let images = match object.get("images") {
            Some(Value::Sequence(s)) => s.clone(),
            None => vec![],
            s => {
                return Err(format!(
                    "Expected \"images\" to be an array, got {:?} instead",
                    s
                ))
            }
        };

        for image_val in images {
            let path = match image_val.get("path") {
                Some(Value::String(s)) => s,
                s => {
                    return Err(format!(
                        "Expected \"path\" to be a string, got {:?} instead.",
                        s
                    ));
                }
            };
            let name = match image_val.get("name") {
                Some(Value::String(s)) => CString::new(s.as_str()).unwrap(),
                s => return Err(format!("Expected name to be a string, got {:?} instead", s)),
            };

            if let Some(_) = buffers.get(&name) {
                return Err(format!(
                    "Texture {:?} already exists, please try a different name",
                    name
                ));
            }

            let dyn_image = ImageReader::open(path)
                .expect(format!("Failed to load image {:?} at {}", name, path).as_str())
                .decode()
                .expect(format!("Failed to decode image {:?} at {}", name, path).as_str());
            let tex = Box::new(make_texture_from_image(dyn_image));
            buffers.insert(name, tex);
        }

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
                    Box::new(FrameBuffer::with_params(
                        width as _,
                        height as _,
                        stage.repeat,
                        stage.linear,
                        stage.mipmap,
                        stage.float,
                    ))
                }
                StageKind::Comp {
                    tex_type, tex_dim, ..
                } => make_image(&tex_dim[..(tex_type as _)]),
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

    pub fn update(
        &mut self,
        path: impl AsRef<Path>,
        screen_size: (u32, u32),
    ) -> Result<(), String> {
        let reader = match std::fs::File::open(path) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let object: Value = match serde_yaml::from_reader(reader) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let passes = match object.get("stages") {
            Some(Value::Sequence(s)) => s.clone(),
            None => return Err("Required field \"stages\" not found".to_string()),
            s => return Err(format!("Expected \"stages\" to be an array, got {:?}", s)),
        };

        // get fft texture size
        let fft_size = match object.get("fft_size") {
            None => 8192,
            Some(Value::Number(n)) => {
                let n = n.as_u64().unwrap();
                if n.is_power_of_two() {
                    return Err(format!(
                        "Expected \"fft_size\" to be a power of 2, got: {:?}",
                        n
                    ));
                }
                n
            }
            s => return Err(format!("Expected \"fft_size\" to be number, got: {:?}", s)),
        };

        // parse stages
        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_yaml(pass)?;
            stages.push(stage);
        }

        // put buffers into hashmap
        let buffers = &mut self.buffers;

        // add audio samples to buffers
        let audio_samples_texture = Texture1D::with_params(
            [fft_size as _],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        let raw_spectrums_texture = Texture1D::with_params(
            [(fft_size / 2) as _],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        let spectrums_texture = Texture1D::with_params(
            [100],
            gl::NEAREST,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            TextureFormat::RG32F,
        );

        buffers.insert(
            CString::new("samples").unwrap(),
            Box::new(audio_samples_texture),
        );

        buffers.insert(
            CString::new("raw_spectrum").unwrap(),
            Box::new(raw_spectrums_texture),
        );

        buffers.insert(
            CString::new("spectrum").unwrap(),
            Box::new(spectrums_texture),
        );

        // add noise texture
        let noise = Box::new(make_noise());
        buffers.insert(CString::new("noise").unwrap(), noise);

        let images = match object.get("images") {
            Some(Value::Sequence(s)) => s.clone(),
            None => vec![],
            s => {
                return Err(format!(
                    "Expected \"images\" to be an array, got {:?} instead",
                    s
                ))
            }
        };

        for image_val in images {
            let path = match image_val.get("path") {
                Some(Value::String(s)) => s,
                s => {
                    return Err(format!(
                        "Expected \"path\" to be a string, got {:?} instead.",
                        s
                    ));
                }
            };
            let name = match image_val.get("name") {
                Some(Value::String(s)) => CString::new(s.as_str()).unwrap(),
                s => return Err(format!("Expected name to be a string, got {:?} instead", s)),
            };

            if let Some(_) = buffers.get(&name) {
                continue;
            }

            let dyn_image = ImageReader::open(path)
                .expect(format!("Failed to load image {:?} at {}", name, path).as_str())
                .decode()
                .expect(format!("Failed to decode image {:?} at {}", name, path).as_str());
            let tex = Box::new(make_texture_from_image(dyn_image));
            buffers.insert(name, tex);
        }

        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            // check if target exists already
            if let Some(tex) = buffers.get(target) {
                if Some(tex.resolution()) != stage.resolution() {
                    continue;
                }

                continue;
            }

            // create textures
            let texture: Box<dyn Texture> = match stage.kind {
                StageKind::Frag { res } | StageKind::Vert { res, .. } => {
                    let (width, height) = res.unwrap_or(screen_size);
                    Box::new(FrameBuffer::with_params(
                        width as _,
                        height as _,
                        stage.repeat,
                        stage.linear,
                        stage.mipmap,
                        stage.float,
                    ))
                }
                StageKind::Comp {
                    tex_type, tex_dim, ..
                } => make_image(&tex_dim[..(tex_type as _)]),
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
        self.stages = stages;
        Ok(())
    }

    pub fn resize_buffers(&mut self, width: u32, height: u32) {
        for stage in self.stages.iter() {
            match &stage.target {
                Some(s) => {
                    let tex = match stage.kind {
                        StageKind::Frag { res: None, .. } | StageKind::Vert { res: None, .. } => {
                            FrameBuffer::with_params(
                                width,
                                height,
                                stage.repeat,
                                stage.linear,
                                stage.mipmap,
                                stage.float,
                            )
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
