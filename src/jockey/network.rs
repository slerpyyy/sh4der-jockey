use super::*;
use ndi::{self, FindCreateError, FindSourcesTimeout};
use std::{
    iter::FromIterator,
    sync::{Arc, Mutex},
    thread,
};

static NDI_RECEIVER_NAME: &'static str = "Sh4derJockey";

#[derive(Debug)]
pub struct Ndi {
    sources: Arc<Mutex<Vec<ndi::Source>>>,
    videos: HashMap<String, Arc<Mutex<image::DynamicImage>>>,
    searching: bool,
}

impl Ndi {
    pub fn new() -> Self {
        Self {
            sources: Default::default(),
            videos: HashMap::new(),
            searching: false,
        }
    }

    fn search_sources(&self, blocking: bool) {
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
                    let name = source.get_name();
                    if name
                        .chars()
                        .filter(|&c| c != std::char::REPLACEMENT_CHARACTER)
                        .next()
                        .is_none()
                    {
                        continue;
                    }

                    let pos = sources.binary_search_by_key(&name, |s| s.get_name());
                    if let Err(index) = pos {
                        sources.insert(index, source);
                    }
                }

                //log::info!("Found NDI sources: {:?}", sources);

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
        let size = video.line_stride_in_bytes().unwrap() * video.height();
        let slice = unsafe { std::slice::from_raw_parts(video.p_data(), size as _) };
        let vec = Vec::from_iter(slice.to_owned());

        match video.four_cc() {
            ndi::FourCCVideoType::BGRA => {
                let buf = image::ImageBuffer::<image::Bgra<u8>, Vec<_>>::from_vec(
                    video.width(),
                    video.height(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageBgra8(buf)
            }
            ndi::FourCCVideoType::BGRX => {
                let buf = image::ImageBuffer::<image::Bgr<u8>, Vec<_>>::from_vec(
                    video.width(),
                    video.height(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageBgr8(buf)
            }
            ndi::FourCCVideoType::RGBA => {
                let buf = image::ImageBuffer::<image::Rgba<u8>, Vec<_>>::from_vec(
                    video.width(),
                    video.height(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageRgba8(buf)
            }
            ndi::FourCCVideoType::RGBX => {
                let buf = image::ImageBuffer::<image::Rgb<u8>, Vec<_>>::from_vec(
                    video.width(),
                    video.height(),
                    vec,
                )
                .unwrap();

                image::DynamicImage::ImageRgb8(buf)
            }
            _ => panic!("Failed to convert image"),
        }
    }

    pub fn connect(&mut self, requested: &[&str]) -> Result<(), ndi::RecvCreateError> {
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

        log::info!("Found NDI sources: {:?}", sources);

        let src: Vec<(String, &ndi::Source)> = sources
            .iter()
            .filter_map(|src| {
                let src_name = src.get_name();
                for &pat in requested {
                    if src_name.contains(pat) {
                        return Some((pat.into(), src));
                    }
                }
                None
            })
            .collect();

        log::info!(
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

            log::info!("Connected to NDI source: {}", source.get_name());

            let weak = Arc::downgrade(&video);
            thread::spawn(move || {
                loop {
                    if weak.strong_count() == 0 {
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
                        break;
                    }
                }

                log::info!("Terminating RECV thread");
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
