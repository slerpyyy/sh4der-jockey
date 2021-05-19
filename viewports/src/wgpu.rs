use crate::{Manager, Viewport};
use imgui::TextureId;
use imgui_wgpu::{RendererConfig, TextureConfig};
use std::collections::HashMap;
use winit::window::{Window, WindowId};

pub struct Wgpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    renderer: imgui_wgpu::Renderer,
}

pub struct ImageData {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
    format: wgpu::TextureFormat,
}
impl ImageData {
    pub fn new(width: u32, height: u32, bytes: Vec<u8>, format: wgpu::TextureFormat) -> Self {
        Self {
            width,
            height,
            bytes,
            format,
        }
    }
    #[cfg(feature = "from-image")]
    pub fn from_image(image: image::DynamicImage) -> Self {
        use image::GenericImageView;
        use wgpu::TextureFormat;
        let (width, height) = image.dimensions();
        let format = Outlet::format();
        let bytes = match format {
            TextureFormat::Bgra8Unorm => image.to_bgra().into_raw(),
            TextureFormat::Rgba8Unorm => image.to_rgba().into_raw(),
            _ => unimplemented!(),
        };
        Self {
            width,
            height,
            bytes,
            format,
        }
    }
}

impl Wgpu {
    pub fn new(imgui: &mut imgui::Context, device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let config = RendererConfig {
            texture_format: Outlet::format(),
            ..RendererConfig::new_srgb()
        };
        let renderer = imgui_wgpu::Renderer::new(imgui, &device, &queue, config);
        Self {
            device,
            queue,
            renderer,
        }
    }
    pub fn upload_image(&mut self, data: &ImageData, replace: Option<TextureId>) -> TextureId {
        let texture_config = TextureConfig {
            size: wgpu::Extent3d {
                width: data.width,
                height: data.height,
                ..Default::default()
            },
            format: Some(data.format),
            ..Default::default()
        };

        let texture = imgui_wgpu::Texture::new(&self.device, &self.renderer, texture_config);

        texture.write(&self.queue, &data.bytes, data.width, data.height);
        if let Some(id) = replace {
            self.renderer.textures.replace(id, texture);
            id
        } else {
            self.renderer.textures.insert(texture)
        }
    }
}

#[derive(Debug)]
pub struct Outlet {
    surface: wgpu::Surface,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: Option<wgpu::SwapChain>,
}
impl Outlet {
    fn new(surface: wgpu::Surface) -> Self {
        Outlet {
            surface,
            sc_desc: Self::desc(),
            swap_chain: None,
        }
    }
    fn desc() -> wgpu::SwapChainDescriptor {
        wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: Self::format(),
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Fifo,
        }
    }
    fn format() -> wgpu::TextureFormat {
        wgpu::TextureFormat::Bgra8Unorm
    }
}

pub struct WgpuManager {
    viewports: HashMap<WindowId, WgpuViewport>,
    instance: wgpu::Instance,
}

impl Manager for WgpuManager {
    type Renderer = Wgpu;
    type Viewport = WgpuViewport;

    fn viewport(&self, wid: WindowId) -> Option<&Self::Viewport> {
        self.viewports.get(&wid)
    }
    fn viewport_mut(&mut self, wid: WindowId) -> Option<&mut Self::Viewport> {
        self.viewports.get_mut(&wid)
    }
    fn add_window(&mut self, window: Window) -> WindowId {
        let wid = window.id();
        let surface = unsafe { self.instance.create_surface(&window) };
        let viewport = WgpuViewport::with_surface(window, surface);
        if self.viewports.insert(wid, viewport).is_some() {
            panic!("Trying to add window with same WindowId twice");
        }
        wid
    }
    #[track_caller]
    fn destroy(&mut self, wid: WindowId) {
        let _ = self.viewports.remove(&wid).expect("No window to destroy");
    }
}

impl WgpuManager {
    pub fn new(instance: wgpu::Instance) -> Self {
        let viewports = HashMap::new();
        Self {
            viewports,
            instance,
        }
    }
    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }
    pub fn reqwest_redraws(&self) {
        for viewport in self.viewports.values() {
            viewport.window().request_redraw();
        }
    }
    pub fn viewports_iter(&self) -> impl Iterator<Item = (&WindowId, &WgpuViewport)> {
        self.viewports.iter()
    }
}

pub struct WgpuViewport {
    window: Window,
    outlet: Outlet,
}
impl WgpuViewport {
    fn with_surface(window: Window, surface: wgpu::Surface) -> Self {
        Self {
            window,
            outlet: Outlet::new(surface),
        }
    }
    fn get_current_frame(
        &mut self,
        device: &wgpu::Device,
    ) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
        if self.outlet.swap_chain.is_none() {
            self.create_swap_chain(device);
        }
        self.outlet.swap_chain.as_mut().unwrap().get_current_frame()
    }
    fn create_swap_chain(&mut self, device: &wgpu::Device) {
        let outlet = &mut self.outlet;
        let size = self.window.inner_size();
        outlet.sc_desc.width = size.width;
        outlet.sc_desc.height = size.height;
        outlet.swap_chain = Some(device.create_swap_chain(&outlet.surface, &outlet.sc_desc));
    }
    pub fn surface(&self) -> &wgpu::Surface {
        &self.outlet.surface
    }
}

impl Viewport for WgpuViewport {
    type Renderer = Wgpu;
    fn window(&self) -> &Window {
        &self.window
    }
    fn on_resize(&mut self) {
        self.outlet.swap_chain = None;
    }
    fn on_draw(&mut self, wgpu: &mut Wgpu, draw_data: &imgui::DrawData) {
        let mut encoder: wgpu::CommandEncoder = wgpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let frame = match self.get_current_frame(&wgpu.device) {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {:?}", e);
                return;
            }
        };

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.output.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear_color),
                    store: true,
                },
            }],
            depth_stencil_attachment: None,
        });

        wgpu.renderer
            .render(draw_data, &wgpu.queue, &wgpu.device, &mut rpass)
            .expect("Rendering failed");
        drop(rpass);
        wgpu.queue.submit(Some(encoder.finish()));
    }
}
