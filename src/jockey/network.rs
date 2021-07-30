use super::*;
use ndi::{self, FindCreateError, FindSourcesTimeout};
use std::{
    iter::FromIterator,
    sync::{Arc, Mutex},
    thread,
};

static NDI_RECEIVER_NAME: &'static str = "Sh4derJockey";

pub struct Ndi {
    sources: Arc<Mutex<Vec<ndi::Source>>>,
    videos: HashMap<String, Arc<Mutex<image::DynamicImage>>>,
    searching: bool,
}

impl Ndi {
    pub fn new(requested: &[String]) -> Self {
        let sources = Default::default();
        let videos = HashMap::new();
        let searching = false;
        let mut this = Self { sources, videos, searching };

        if let Err(e) = this.connect(requested) {
            eprintln!("Failed to connect to NDI sources: {}", e);
        }

        this
    }

    pub fn search_sources(&self, blocking: bool) {
        let sources = self.sources.clone();
        let handle = thread::spawn(move || -> Result<(), FindCreateError> {
            let find_local = ndi::FindBuilder::new().show_local_sources(true).build()?;
            let find_remote = ndi::FindBuilder::new().show_local_sources(false).build()?;

            loop {
                if !blocking {
                    thread::sleep(Duration::from_secs(2));
                }

                let locals = match find_local.current_sources(1000) {
                    Ok(s) => s,
                    Err(FindSourcesTimeout) => vec![],
                };

                let remotes = match find_remote.current_sources(1000) {
                    Ok(s) => s,
                    Err(FindSourcesTimeout) => vec![],
                };

                let mut sources = sources.lock().unwrap();
                for source in locals.into_iter().chain(remotes) {
                    let name = source.get_name().ok();
                    let pos = sources.binary_search_by_key(&name, |s| s.get_name().ok());
                    if let Err(index) = pos {
                        sources.insert(index, source);
                    }
                }

                if blocking {
                    return Ok(());
                }
            }
        });

        if blocking {
            handle.join().unwrap().unwrap();
        }
    }

    fn convert_format(video: ndi::VideoData) -> image::DynamicImage {
        let size = video.line_stride_in_bytes().unwrap() * video.yres();
        let slice = unsafe { std::slice::from_raw_parts(video.p_data(), size as _) };
        let vec = Vec::from_iter(slice.to_owned());

        match video.four_cc() {
            ndi::FourCCVideoType::BGRA => {
                let buf = image::ImageBuffer::<image::Bgra<u8>, Vec<_>>::from_vec(
                    video.xres(),
                    video.yres(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageBgra8(buf)
            }
            ndi::FourCCVideoType::BGRX => {
                let buf = image::ImageBuffer::<image::Bgr<u8>, Vec<_>>::from_vec(
                    video.xres(),
                    video.yres(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageBgr8(buf)
            }
            ndi::FourCCVideoType::RGBA => {
                let buf = image::ImageBuffer::<image::Rgba<u8>, Vec<_>>::from_vec(
                    video.xres(),
                    video.yres(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageRgba8(buf)
            }
            ndi::FourCCVideoType::RGBX => {
                let buf = image::ImageBuffer::<image::Rgb<u8>, Vec<_>>::from_vec(
                    video.xres(),
                    video.yres(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageRgb8(buf)
            }
            _ => panic!("Failed to convert image"),
        }
    }

    pub fn connect(&mut self, requested: &[String]) -> Result<(), ndi::RecvCreateError> {
        if requested.is_empty() {
            return Ok(());
        }

        let sources = if self.searching {
            self.sources.lock().unwrap()
        } else {
            self.search_sources(true);

            // take lock before spawning the search thread
            let res = self.sources.lock().unwrap();
            self.search_sources(false);
            self.searching = true;
            res
        };

        println!("Found sources: {:?}", sources);

        let src: Vec<(String, &ndi::Source)> = sources
            .iter()
            .filter_map(|src| {
                let src_name = src.get_name().ok()?;
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

        self.videos
            .retain(|pre_req, _| src.iter().find(|(req, _)| req == pre_req).is_some());

        for (req, source) in src {
            if self.videos.get(&req).is_some() {
                continue;
            }

            let mut recv = ndi::RecvBuilder::new()
                .color_format(ndi::RecvColorFormat::RGBX_RGBA)
                .ndi_recv_name(NDI_RECEIVER_NAME.to_string())
                .build()?;

            recv.connect(&source);

            let video = Arc::new(Mutex::new(image::DynamicImage::ImageRgba8(
                image::ImageBuffer::new(1, 1),
            )));

            self.videos.insert(req, Arc::clone(&video));

            println!(
                "Connected to NDI source: {}",
                source.get_name().unwrap_or_else(|_| "<no-name>".into())
            );

            let weak = Arc::downgrade(&video);
            thread::spawn(move || loop {
                if weak.strong_count() == 0 {
                    println!("Ending RECV loop");
                    break;
                }

                let mut video_data = None;
                if recv.capture_video(&mut video_data, 1000) != ndi::FrameType::Video {
                    continue;
                }

                let img = match video_data {
                    Some(video) => Ndi::convert_format(video).flipv(),
                    _ => continue,
                };

                if let Some(strong) = weak.upgrade() {
                    *strong.lock().unwrap() = img;
                } else {
                    println!("Ending RECV loop");
                    break;
                }
            });
        }

        Ok(())
    }

    pub fn update_texture(&self, tex_name: &String, tex: &mut Texture2D) {
        if let Some(video) = self.videos.get(tex_name) {
            let video = video.lock().unwrap().to_rgba8();
            if tex.resolution() != [video.width(), video.height(), 0] {
                *tex = Texture2D::with_params(
                    [video.width(), video.height()],
                    tex.min_filter,
                    tex.mag_filter,
                    tex.wrap_mode,
                    tex.format,
                    tex.mipmap,
                    video.as_ptr() as _,
                );
            } else {
                tex.write(video.as_ptr() as _);
            }
        }
    }
}
