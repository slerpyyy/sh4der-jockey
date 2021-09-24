use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    path::Path,
    rc::Rc,
};

use async_std::task::yield_now;
use serde_yaml::Value;

use super::uniforms::*;
use crate::{jockey::*, util::Cache};

pub type PipelinePartial = Box<dyn Future<Output = Result<(Pipeline, UpdateRequest), String>>>;

#[derive(Debug, Clone)]
pub struct UpdateRequest {
    pub audio_samples: usize,
    pub smoothing_attack: f32,
    pub smoothing_decay: f32,
}

impl Default for UpdateRequest {
    fn default() -> Self {
        Self {
            audio_samples: AUDIO_SAMPLES,
            smoothing_attack: FFT_ATTACK,
            smoothing_decay: FFT_DECAY,
        }
    }
}

/// The rendering pipeline struct
///
/// This struct holds the structure of the rendering pipeline. Note that it
/// does not render anything itself, it merely holds the information and takes
/// care of resource management.
#[derive(Debug)]
pub struct Pipeline {
    pub stages: Vec<Stage>,
    pub buffers: HashMap<CString, Rc<dyn Texture>>,
    pub requested_ndi_sources: HashMap<CString, String>,
}

impl Pipeline {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            buffers: HashMap::new(),
            requested_ndi_sources: HashMap::new(),
        }
    }

    pub fn splash_screen() -> Self {
        const SPLASH_FRAG: &str = include_str!("shaders/splash.frag");

        let sh_ids = vec![
            compile_shader(PASS_VERT, gl::VERTEX_SHADER).unwrap(),
            compile_shader(SPLASH_FRAG, gl::FRAGMENT_SHADER).unwrap(),
        ];

        let prog_id = link_program(&sh_ids).unwrap();

        let stages = vec![Stage {
            prog_id,
            target: None,
            kind: StageKind::Frag {},
            sh_ids,
            deps: Vec::new(),
            unis: HashMap::new(),
            perf: RunningAverage::new(),
            builder: TextureBuilder::new(),
        }];

        Self {
            stages,
            buffers: HashMap::new(),
            requested_ndi_sources: HashMap::new(),
        }
    }

    pub async fn load(
        path: impl AsRef<Path>,
        screen_size: (u32, u32),
    ) -> Result<(Self, UpdateRequest), String> {
        let empty_cache = HashMap::new();
        Pipeline::from_file_with_cache(path, screen_size, &empty_cache).await
    }

    async fn from_file_with_cache(
        path: impl AsRef<Path>,
        screen_size: (u32, u32),
        cache: &HashMap<CString, Rc<dyn Texture>>,
    ) -> Result<(Self, UpdateRequest), String> {
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
    ) -> Result<(Self, UpdateRequest), String> {
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
            mut spectrum_integrated_opts,
            mut spectrum_smooth_integrated_opts,
            audio_samples,
            smoothing_attack,
            smoothing_decay,
        ) = match object.get("audio") {
            None => (
                TextureBuilder::new(),
                TextureBuilder::new(),
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
                let spectrum_integrated_opts = match object.get("spectrum_integrated") {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };
                let spectrum_smooth_integrated_opts = match object.get("spectrum_smooth_integrated")
                {
                    Some(s) => TextureBuilder::parse(s, false, true)?,
                    None => TextureBuilder::new(),
                };

                (
                    samples_opts,
                    raw_spectrum_opts,
                    spectrum_opts,
                    smooth_spectrum_opts,
                    spectrum_integrated_opts,
                    spectrum_smooth_integrated_opts,
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

        spectrum_integrated_opts
            .set_resolution(vec![100 as _; 1])
            .set_channels(2)
            .set_float(true);

        spectrum_smooth_integrated_opts
            .set_resolution(vec![100 as _; 1])
            .set_channels(2)
            .set_float(true);

        // add audio samples to buffers
        buffers.insert(SAMPLES_NAME.clone(), samples_opts.build_texture());

        buffers.insert(SPECTRUM_RAW_NAME.clone(), raw_spectrum_opts.build_texture());

        buffers.insert(SPECTRUM_NAME.clone(), spectrum_opts.build_texture());

        buffers.insert(
            SPECTRUM_SMOOTH_NAME.clone(),
            smooth_spectrum_opts.build_texture(),
        );

        buffers.insert(
            SPECTRUM_INTEGRATED_NAME.clone(),
            spectrum_integrated_opts.build_texture(),
        );

        buffers.insert(
            SPECTRUM_SMOOTH_INTEGRATED_NAME.clone(),
            spectrum_smooth_integrated_opts.build_texture(),
        );

        {
            // add noise texture
            let noise_name = NOISE_NAME.clone();
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
            None => Vec::new(),
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

        //parse ndi section
        let ndi_sources = match object.get("ndi") {
            Some(Value::Sequence(s)) => s.clone(),
            None => Vec::new(),
            Some(s) => {
                return Err(format!(
                    "Expected \"ndi\" to be an array, got {:?} instead.",
                    s
                ));
            }
        };

        let mut requested_ndi_sources = HashMap::new();
        for src in ndi_sources {
            let source = match src.get("source") {
                Some(Value::String(s)) => s.clone(),
                s => {
                    return Err(format!(
                        "Expected ndi.source to be a string, got {:?} instead",
                        s
                    ))
                }
            };
            let name = match src.get("name") {
                Some(Value::String(s)) => CString::new(s.clone()).unwrap(),
                s => {
                    return Err(format!(
                        "Expected ndi.name to be a string, got {:?} instead",
                        s
                    ))
                }
            };

            if buffers.get(&name).is_some() {
                return Err(format!(
                    "Texture {:?} already exists, please try a different name",
                    name
                ));
            }

            let tex = TextureBuilder::parse(&src, false, true)?
                .set_float(false)
                .set_resolution(vec![1, 1])
                .build_texture();

            requested_ndi_sources.insert(name.clone(), source);
            buffers.insert(name, tex);
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
        let mut used_buffers = HashSet::new();
        for stage in stages.iter_mut() {
            for tex_name in buffers.keys() {
                // try to locate the uniform in the program
                let loc = unsafe { gl::GetUniformLocation(stage.prog_id, tex_name.as_ptr()) };

                // add uniform to list of dependencies
                if loc != -1 {
                    stage.deps.push(tex_name.clone());
                    used_buffers.insert(tex_name.clone());
                }
            }

            yield_now().await;
        }

        // remove unnecessary buffers
        buffers.retain(|name, _| {
            let needed = used_buffers.contains(name);
            if !needed {
                requested_ndi_sources.remove(name);
            }
            needed
        });

        Ok((
            Self {
                stages,
                buffers,
                requested_ndi_sources,
            },
            UpdateRequest {
                audio_samples,
                smoothing_attack,
                smoothing_decay,
            },
        ))
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
