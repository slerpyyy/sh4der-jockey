extern crate ndi;
use std::{
    convert::TryInto,
    sync::{Arc, Mutex},
    thread,
};

use ndi::VideoData;

use super::*;

mod errors {
    error_chain! {
        foreign_links {
            NDI(ndi::NDIError);
            Other(std::str::Utf8Error);
        }
    }
}

use errors::*;

static NDI_RECEIVER_NAME: &'static str = "Sh4derJockey";

pub struct Ndi {
    sources: Arc<Mutex<Vec<ndi::Source>>>,
    videos: HashMap<String, (Arc<Mutex<bool>>, Arc<Mutex<VideoData>>)>,
}

impl Ndi {
    pub fn new(requested: &[String]) -> Self {
        let sources = Arc::new(Mutex::new(vec![]));
        let videos = HashMap::new();

        let mut this = Self { sources, videos };

        this.start_search();

        if let Err(e) = this.connect(requested) {
            eprintln!("Failed to connect to NDI sources: {}", e);
        }

        this
    }

    pub fn start_search(&self) {
        let mutex = self.sources.clone();
        thread::spawn(move || -> Result<()> {
            let find_local = ndi::FindBuilder::new().show_local_sources(true).build()?;
            let find_remote = ndi::FindBuilder::new().show_local_sources(false).build()?;
            loop {
                thread::sleep(Duration::from_secs(2));
                let mut sources = mutex.lock().unwrap();
                let mut locals = match find_local.current_sources(1000) {
                    Ok(s) => s,
                    Err(ndi::NDIError::FindSourcesTimeout) => vec![],
                    _ => {
                        eprintln!("Something funky happened in NDI find");
                        vec![]
                    }
                };
                let mut remotes = match find_remote.current_sources(1000) {
                    Ok(s) => s,
                    Err(ndi::NDIError::FindSourcesTimeout) => vec![],
                    _ => {
                        eprintln!("Something funky happened in NDI find");
                        vec![]
                    }
                };
                locals.append(&mut remotes);
                *sources = locals;
            }
        });
    }

    pub fn connect(&mut self, requested: &[String]) -> Result<()> {
        let sources = self.sources.lock().unwrap();
        println!("{:?}", sources);
        let src: Vec<(String, &ndi::Source)> = sources
            .iter()
            .filter_map(|src| {
                let src_name = src.get_name().unwrap_or_else(|_| String::new());
                for pat in requested {
                    if src_name.contains(pat) {
                        return Some((pat.clone(), src));
                    }
                }
                None
            })
            .collect();

        println!(
            "Found {} of {} requested NDI sources",
            src.len(),
            requested.len()
        );

        let mut dump = vec![];
        for (pre_req, (active, _)) in self.videos.iter() {
            let mut is_active = active.lock().unwrap();
            let mut matched = false;
            for (req, _) in src.iter() {
                matched = matched || req == pre_req;
            }
            if !matched {
                dump.push(pre_req.clone());
                *is_active = false;
            }
        }

        for k in dump {
            self.videos.remove(&k);
        }

        for (req, source) in src {
            if let Some(_) = self.videos.get(&req) {
                continue;
            }

            let mut recv = ndi::RecvBuilder::new()
                .color_format(ndi::RecvColorFormat::RGBX_RGBA)
                .ndi_recv_name(NDI_RECEIVER_NAME.to_string())
                .build()?;
            recv.connect(&source);
            let arc = Arc::new(Mutex::new(VideoData::new()));
            let active = Arc::new(Mutex::new(true));
            self.videos.insert(req, (active.clone(), arc.clone()));

            println!("Connected to NDI source: {}", source.get_name()?);

            thread::spawn(move || loop {
                // seems to deadlock otherwise
                thread::sleep(Duration::from_millis(1));
                if !*active.lock().unwrap() {
                    println!("Ending recv loop");
                    break;
                }
                let mut video_data = None;
                let frame_type = recv.capture_video(&mut video_data, 1000);
                if frame_type == ndi::FrameType::Video {
                    if let Some(video) = video_data {
                        let mut lock = arc.lock().unwrap();
                        *lock = video;
                    }
                }
            });
        }

        Ok(())
    }

    pub fn update_texture(&self, tex_name: &String, tex: &mut Texture2D) {
        if let Some((_, video)) = self.videos.get(tex_name) {
            let video = video.lock().unwrap();
            if tex.resolution() != [video.xres(), video.yres(), 0] {
                *tex = Texture2D::with_params(
                    [video.xres(), video.yres()],
                    tex.min_filter,
                    tex.mag_filter,
                    tex.wrap_mode,
                    tex.format,
                    tex.mipmap,
                    video.p_data() as _,
                );
            } else {
                tex.write(video.p_data() as _);
            }
        }
    }
}
