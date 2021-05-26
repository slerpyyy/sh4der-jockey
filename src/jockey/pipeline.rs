use crate::{jockey::*, util::Cache};
use async_std::task::yield_now;
use serde_yaml::Value;
use std::{collections::HashMap, ffi::CString, path::Path, rc::Rc};

/// The rendering pipeline struct
///
/// This struct holds the structure of the rendering pipeline. Note that it
/// does not render anything itself, it merely holds the information and takes
/// care of resource management.
#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<CString, Rc<dyn Texture>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            buffers: HashMap::new(),
        }
    }

    pub async fn load(path: impl AsRef<Path>, screen_size: (u32, u32)) -> Result<Self, String> {
        let empty_cache = HashMap::new();
        Pipeline::from_file_with_cache(path, screen_size, &empty_cache).await
    }

    /*
    #[allow(dead_code)]
    pub fn update(
        path: impl AsRef<Path>,
        screen_size: (u32, u32),
        old: &Self,
    ) -> Result<Self, String> {
        Pipeline::from_file_with_cache(path, screen_size, &old.buffers)
    }
    */

    async fn from_file_with_cache(
        path: impl AsRef<Path>,
        screen_size: (u32, u32),
        cache: &HashMap<CString, Rc<dyn Texture>>,
    ) -> Result<Self, String> {
        let reader = match std::fs::File::open(path) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        let object = match serde_yaml::from_reader(reader) {
            Ok(s) => s,
            Err(e) => return Err(e.to_string()),
        };

        Pipeline::from_yaml_with_cache(object, screen_size, cache).await
    }

    async fn from_yaml_with_cache(
        object: Value,
        screen_size: (u32, u32),
        cache: &HashMap<CString, Rc<dyn Texture>>,
    ) -> Result<Self, String> {
        let mut buffers = HashMap::<CString, Rc<dyn Texture>>::new();

        // init global texture cache
        Cache::init();

        // get fft texture size
        let fft_size = match object.get("fft_size") {
            None => AUDIO_SAMPLES as _,
            Some(Value::Number(n)) => {
                match n.as_u64() {
                    Some(n) if n.is_power_of_two() => n,
                    _ => return Err(format!(
                        "Expected \"fft_size\" to be a power of 2, got: {:?}",
                        n
                    )),
                }
            }
            s => return Err(format!("Expected \"fft_size\" to be number, got: {:?}", s)),
        };

        // add audio samples to buffers
        buffers.insert(
            CString::new("samples").unwrap(),
            Rc::new(Texture1D::with_params(
                [fft_size as _],
                gl::NEAREST,
                gl::NEAREST,
                gl::CLAMP_TO_EDGE,
                TextureFormat::RG32F,
                std::ptr::null(),
            )),
        );

        buffers.insert(
            CString::new("raw_spectrum").unwrap(),
            Rc::new(Texture1D::with_params(
                [(fft_size / 2) as _],
                gl::NEAREST,
                gl::NEAREST,
                gl::CLAMP_TO_EDGE,
                TextureFormat::RG32F,
                std::ptr::null(),
            )),
        );

        buffers.insert(
            CString::new("spectrum").unwrap(),
            Rc::new(Texture1D::with_params(
                [100],
                gl::NEAREST,
                gl::NEAREST,
                gl::CLAMP_TO_EDGE,
                TextureFormat::RG32F,
                std::ptr::null(),
            )),
        );

        {
            // add noise texture
            let noise_name = CString::new("noise").unwrap();
            let noise = match cache.get(&noise_name) {
                Some(old) => Rc::clone(old),
                None => Rc::new(make_noise()),
            };
            buffers.insert(noise_name, noise);
        }

        yield_now().await;

        // parse images section
        let images = match object.get("images") {
            Some(Value::Sequence(s)) => s.clone(),
            None => vec![],
            s => return Err(format!("Expected \"images\" to be an array, got {:?}", s)),
        };

        // parse images
        for image in images {
            let path = match image.get("path") {
                Some(Value::String(s)) => s,
                s => {
                    return Err(format!("Expected \"path\" to be a string, got {:?}", s));
                }
            };

            let name = match image.get("name") {
                Some(Value::String(s)) => CString::new(s.as_str()).unwrap(),
                s => return Err(format!("Expected \"name\" to be a string, got {:?}", s)),
            };

            // check if name is already in use
            if buffers.get(&name).is_some() {
                return Err(format!(
                    "Texture {:?} already exists, please try a different name",
                    name
                ));
            }

            // fetch texture from global cache
            let tex = match Cache::fetch(path) {
                Some(cached_tex) => cached_tex,
                None => match Cache::load(path.clone()) {
                    Some(s) => s,
                    None => return Err(format!("Failed to load image {:?} at {:?}", name, path)),
                },
            };

            buffers.insert(name, tex);

            yield_now().await;
        }

        // parse stages section
        let passes = match object.get("stages") {
            Some(Value::Sequence(s)) => s.clone(),
            None => return Err("Required field \"stages\" not found".to_string()),
            s => return Err(format!("Expected \"stages\" to be an array, got {:?}", s)),
        };

        // parse stages
        let mut stages = Vec::with_capacity(passes.len());
        for pass in passes {
            let stage = Stage::from_yaml(pass)?;
            stages.push(stage);

            yield_now().await;
        }

        // create render targets for stages
        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            // check if target exists already
            if let Some(tex) = buffers.get(target) {
                let res = stage.resolution();
                if Some(tex.resolution()) != res || res.is_none() {
                    return Err(format!(
                        "Texture {:?} already has a different resolution",
                        target
                    ));
                }

                continue;
            }

            // create textures
            let texture: Rc<dyn Texture> = match stage.kind {
                StageKind::Frag { res } | StageKind::Vert { res, .. } => {
                    let (width, height) = res.unwrap_or(screen_size);
                    Rc::new(FrameBuffer::with_params(
                        width as _,
                        height as _,
                        stage.repeat,
                        stage.linear,
                        stage.mipmap,
                        stage.float,
                    ))
                }
                StageKind::Comp { ref res, .. } => make_image(res.as_slice()),
            };

            // insert texture into hashmap
            buffers.insert(target.clone(), texture);

            yield_now().await;
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
                    self.buffers.insert(s.clone(), Rc::new(tex));
                }
                None => continue,
            }
        }
    }
}
