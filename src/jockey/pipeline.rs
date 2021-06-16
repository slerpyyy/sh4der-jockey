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
    pub audio_samples: usize,
    pub smoothing_attack: f32,
    pub smoothing_decay: f32,
}

impl Pipeline {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            buffers: HashMap::new(),
            audio_samples: AUDIO_SAMPLES,
            smoothing_attack: FFT_ATTACK,
            smoothing_decay: FFT_DECAY,
        }
    }

    pub fn splash_screen() -> Self {
        const SPLASH_VERT: &str = include_str!("shaders/splash.vert");

        let sh_ids = vec![
            compile_shader(SPLASH_VERT, gl::VERTEX_SHADER).unwrap(),
            compile_shader(PASS_FRAG, gl::FRAGMENT_SHADER).unwrap(),
        ];

        let prog_id = link_program(&sh_ids).unwrap();

        let stages = vec![Stage {
            prog_id,
            target: None,
            kind: StageKind::Vert {
                count: 98,
                mode: gl::LINES,
                thickness: 5.0,
            },
            sh_ids,
            deps: Vec::new(),
            perf: RunningAverage::new(),
            builder: TextureBuilder::new(),
        }];

        Self {
            stages,
            buffers: HashMap::new(),
            audio_samples: AUDIO_SAMPLES,
            smoothing_attack: FFT_ATTACK,
            smoothing_decay: FFT_DECAY,
        }
    }

    pub async fn load(path: impl AsRef<Path>, screen_size: (u32, u32)) -> Result<Self, String> {
        let empty_cache = HashMap::new();
        Pipeline::from_file_with_cache(path, screen_size, &empty_cache).await
    }

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
        yield_now().await;

        // init global texture cache
        Cache::init();

        // get fft texture size
        let (
            mut samples_opts,
            mut raw_spectrum_opts,
            mut spectrum_opts,
            mut smooth_spectrum_opts,
            audio_samples,
            smoothing_attack,
            smoothing_decay,
        ) = match object.get("audio") {
            None => (
                TextureBuilder::new(),
                TextureBuilder::new(),
                TextureBuilder::new(),
                TextureBuilder::new(),
                AUDIO_SAMPLES,
                FFT_ATTACK,
                FFT_DECAY,
            ),
            Some(object) => {
                let audio_samples = match object.get("audio_samples") {
                    None => AUDIO_SAMPLES,
                    Some(Value::Number(n)) => match n.as_u64() {
                        Some(n) => n as _,
                        _ => {
                            return Err(format!(
                                "Expected \"audio_samples\" to be a number, got: {:?}",
                                n
                            ))
                        }
                    },
                    s => {
                        return Err(format!(
                            "Expected \"audio_samples\" to be number, got: {:?}",
                            s
                        ))
                    }
                };

                let attack = match object.get("attack") {
                    None => FFT_ATTACK,
                    Some(s) => match s.as_f64() {
                        Some(s) => s as _,
                        _ => {
                            return Err(format!(
                                "Expected \"smoothing\" to be a float, got {:?}",
                                s
                            ))
                        }
                    },
                };
                let decay = match object.get("decay") {
                    None => FFT_DECAY,
                    Some(s) => match s.as_f64() {
                        Some(s) => s as _,
                        _ => {
                            return Err(format!(
                                "Expected \"smoothing\" to be a float, got {:?}",
                                s
                            ))
                        }
                    },
                };

                let samples_opts = match object.get("samples") {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };
                let raw_spectrum_opts = match object.get("spectrum_raw") {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };
                let spectrum_opts = match object.get("spectrum") {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };
                let smooth_spectrum_opts = match object.get("spectrum_smooth") {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };

                (
                    samples_opts,
                    raw_spectrum_opts,
                    spectrum_opts,
                    smooth_spectrum_opts,
                    audio_samples,
                    attack,
                    decay,
                )
            }
        };

        samples_opts
            .set_resolution(vec![audio_samples as _; 1])
            .set_channels(2)
            .set_float(true);

        raw_spectrum_opts
            .set_resolution(vec![(audio_samples / 2) as _; 1])
            .set_channels(2)
            .set_float(true);

        spectrum_opts
            .set_resolution(vec![100 as _; 1])
            .set_channels(2)
            .set_float(true);

        smooth_spectrum_opts
            .set_resolution(vec![100 as _; 1])
            .set_channels(2)
            .set_float(true);

        // add audio samples to buffers
        buffers.insert(
            CString::new("samples").unwrap(),
            samples_opts.build_texture(),
        );

        buffers.insert(
            CString::new("spectrum_raw").unwrap(),
            raw_spectrum_opts.build_texture(),
        );

        buffers.insert(
            CString::new("spectrum").unwrap(),
            spectrum_opts.build_texture(),
        );

        buffers.insert(
            CString::new("spectrum_smooth").unwrap(),
            smooth_spectrum_opts.build_texture(),
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
        for object in images {
            let path = match object.get("path") {
                Some(Value::String(s)) => s,
                s => {
                    return Err(format!("Expected \"path\" to be a string, got {:?}", s));
                }
            };

            let name = match object.get("name") {
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
                None => {
                    let reader = image::io::Reader::open(&path)
                        .map_err(|_| format!("Failed to open image {:?} at {:?}", name, path))?;
                    async_std::task::yield_now().await;

                    let dyn_image = reader
                        .decode()
                        .map_err(|_| format!("Failed to decode image {:?} at {:?}", name, path))?;
                    async_std::task::yield_now().await;

                    let image = dyn_image.flipv().to_rgba8();
                    async_std::task::yield_now().await;

                    let mut builder = TextureBuilder::parse(&object, false, false)?;
                    builder.resolution = vec![image.width(), image.height()];
                    let tex = builder.build_texture_with_data(image.as_raw().as_ptr() as _);
                    async_std::task::yield_now().await;

                    Cache::store(path.clone(), Rc::clone(&tex));
                    tex
                }
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
        let mut res_map = HashMap::new();
        for stage in stages.iter() {
            let target = match &stage.target {
                Some(s) => s,
                None => continue,
            };

            // check if target exists already
            let stage_res = stage.resolution();
            if buffers.contains_key(target) {
                if let Some(&buffer_res) = res_map.get(target.as_c_str()) {
                    // compare against previous stages
                    if buffer_res != stage_res {
                        return Err(format!(
                            "Texture {:?} already has a different resolution",
                            target
                        ));
                    }

                    // don't create the same target twice
                    continue;
                } else {
                    return Err(format!(
                        "Target {:?} is already loaded as an image or build-in texture",
                        target
                    ));
                }
            }

            // record specified stage resolution
            res_map.insert(target.as_c_str(), stage_res);

            // create textures
            let texture: Rc<dyn Texture> = match stage.kind {
                StageKind::Frag { .. } | StageKind::Vert { .. } => {
                    stage.builder.build_double_framebuffer(screen_size)
                }
                StageKind::Comp { .. } => stage.builder.build_image(),
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

            yield_now().await;
        }

        Ok(Self {
            stages,
            buffers,
            audio_samples,
            smoothing_attack,
            smoothing_decay,
        })
    }

    pub fn resize_buffers(&mut self, width: u32, height: u32) {
        for stage in self.stages.iter() {
            if !stage.builder.resolution.is_empty() {
                continue;
            }

            if !matches!(stage.kind, StageKind::Frag { .. } | StageKind::Vert { .. }) {
                panic!("なに the fuck?")
            }

            // get name of stage render target
            let name = match &stage.target {
                Some(s) => s.clone(),
                _ => continue,
            };

            self.buffers.insert(
                name,
                stage.builder.build_double_framebuffer((width, height)),
            );
        }
    }
}
