extern crate ndi;
use std::{
    sync::mpsc::{channel, Receiver},
    thread,
};

use ndi::VideoData;

use super::*;
use crate::util::Texture2D;

static NDI_RECEIVER_NAME: &'static str = "Sh4derJockey";

pub struct Ndi {
    pub texture: Option<Texture2D>,

    find: Option<ndi::Find>,
    queues: Option<Receiver<ndi::VideoData>>,
    current_video: Option<VideoData>,
}

impl Ndi {
    pub fn new(config: &GlobalConfig) -> Self {
        // set show_local_sources to false for remote NDI
        let find = match ndi::FindBuilder::new().show_local_sources(true).build() {
            Ok(find) => Some(find),
            Err(e) => {
                eprintln!("Failed to create NDI finder, {}", e);
                None
            }
        };

        let queues = None;
        let texture = None;
        let current_video = None;

        let mut this = Self {
            find,
            queues,
            texture,
            current_video,
        };

        if !this.find.is_none() {
            this.connect(config).unwrap();
        }

        this
    }

    pub fn connect(&mut self, config: &GlobalConfig) -> Result<(), ndi::NDIError> {
        let find = match self.find.as_ref() {
            Some(x) => Ok(x),
            None => Err(ndi::NDIError::FindCreateError),
        }?;

        let sources = find.current_sources(1000)?;
        let src: Vec<ndi::Source> = sources
            .into_iter()
            .filter(|src| {
                let src = src.get_name().unwrap_or_else(|_| String::new());
                let mut matched = false;
                for pat in config.ndi_sources.iter() {
                    matched = matched || src.contains(pat);
                }
                matched
            })
            .take(1)
            .collect();

        if src.len() != 1 {
            todo!("I can't be bothered right now");
        }

        let source = &src[0];
        let mut recv = ndi::RecvBuilder::new()
            .color_format(ndi::RecvColorFormat::RGBX_RGBA)
            .ndi_recv_name(NDI_RECEIVER_NAME.to_string())
            .build()?;
        recv.connect(source);

        println!("Connected to NDI source: {}", source.get_name().unwrap());

        let (tx, rx) = channel();
        thread::spawn(move || loop {
            let mut video_data = None;
            let frame_type = recv.capture_video(&mut video_data, 1000);
            if frame_type == ndi::FrameType::Video {
                if let Some(video) = video_data {
                    if let Err(e) = tx.send(video) {
                        eprintln!("Failed to send video data: {}", e);
                    }
                }
            }
        });

        self.queues = Some(rx);

        Ok(())
    }

    pub fn handle_input(&mut self) {
        let rx = match &self.queues {
            Some(x) => x,
            None => return,
        };

        let mut last = None;
        for video in rx.try_iter() {
            last = Some(video);
        }
        if let Some(video) = last {
            let tex = match self.texture.as_mut() {
                Some(x) => {
                    if x.resolution() == [video.xres(), video.yres(), 0] {
                        x
                    } else {
                        self.texture = Some(Texture2D::with_params(
                            [video.xres(), video.yres()],
                            gl::LINEAR,
                            gl::LINEAR,
                            gl::CLAMP_TO_EDGE,
                            TextureFormat::RGBA8,
                            std::ptr::null(),
                        ));
                        self.texture.as_mut().unwrap()
                    }
                }
                None => {
                    self.texture = Some(Texture2D::with_params(
                        [video.xres(), video.yres()],
                        gl::LINEAR,
                        gl::LINEAR,
                        gl::CLAMP_TO_EDGE,
                        TextureFormat::RGBA8,
                        std::ptr::null(),
                    ));
                    self.texture.as_mut().unwrap()
                }
            };

            tex.write(video.p_data() as _);
            self.current_video = Some(video);
        }
    }
}
