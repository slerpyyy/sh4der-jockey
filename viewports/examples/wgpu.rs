use futures::executor::block_on;
use imgui::{im_str, Condition, FontSource};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use viewports::{
    wgpu::{Wgpu, WgpuManager},
    Manager, Platform, Viewport,
};

fn setup_first_window<T: 'static>(event_loop: &EventLoop<T>) -> (WgpuManager, WindowId) {
    let instance = wgpu::Instance::new(wgpu::BackendBit::DX12);
    let mut manager = WgpuManager::new(instance);

    let version = env!("CARGO_PKG_VERSION");

    let window = Window::new(&event_loop).unwrap();
    window.set_inner_size(LogicalSize {
        width: 1280.0,
        height: 720.0,
    });
    window.set_outer_position(winit::dpi::PhysicalPosition { x: 0, y: 0 });
    window.set_title(&format!("imgui-wgpu {}", version));

    let main_view = manager.add_window(window);

    (manager, main_view)
}

fn setup_adapter(manager: &WgpuManager, main_view: WindowId) -> wgpu::Adapter {
    block_on(
        manager
            .instance()
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(manager.viewport(main_view).unwrap().surface()),
            }),
    )
    .unwrap()
}

fn setup_imgui(hidpi_factor: f64) -> imgui::Context {
    use imgui::ConfigFlags;
    let mut imgui = imgui::Context::create();

    let io = imgui.io_mut();
    io.config_flags.insert(ConfigFlags::DOCKING_ENABLE);
    io.config_flags.insert(ConfigFlags::VIEWPORTS_ENABLE);

    let font_size = (13.0 * hidpi_factor) as f32;
    io.font_global_scale = (1.0 / hidpi_factor) as f32;
    imgui.fonts().add_font(&[FontSource::DefaultFontData {
        config: Some(imgui::FontConfig {
            oversample_h: 1,
            pixel_snap_h: true,
            size_pixels: font_size,
            ..Default::default()
        }),
    }]);

    imgui
}

fn setup_renderer(adapter: &wgpu::Adapter, imgui: &mut imgui::Context) -> Wgpu {
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
            shader_validation: false,
        },
        None,
    ))
    .unwrap();
    Wgpu::new(imgui, device, queue)
}

fn main() {
    wgpu_subscriber::initialize_default_subscriber(None);

    // Set up window and GPU
    let event_loop = EventLoop::new();

    let (mut manager, main_view) = setup_first_window(&event_loop);

    let adapter = setup_adapter(&manager, main_view);
    dbg!(adapter.get_info());

    let mut imgui = setup_imgui(1.0);

    let mut platform = Platform::init(&mut imgui, manager.viewport(main_view).unwrap());

    let mut renderer = setup_renderer(&adapter, &mut imgui);

    let mut demo_open = true;

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let mut manager_with_loop = manager.with_loop(event_loop);
        match &event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if *window_id == main_view => {
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                platform.frame(&mut imgui, &mut manager_with_loop, |ui, delta| {
                    let window = imgui::Window::new(im_str!("Hello world"));
                    window
                        .size([300.0, 100.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Hello world!"));
                            ui.text(im_str!("This...is...imgui-rs on WGPU with VIEWPORTS!"));
                            ui.separator();
                            let mouse_pos = ui.io().mouse_pos;
                            ui.text(im_str!(
                                "Mouse Position: ({:.1},{:.1})",
                                mouse_pos[0],
                                mouse_pos[1]
                            ));
                        });

                    let window = imgui::Window::new(im_str!("Hello too"));
                    window
                        .size([400.0, 200.0], Condition::FirstUseEver)
                        .position([400.0, 200.0], Condition::FirstUseEver)
                        .build(&ui, || {
                            ui.text(im_str!("Frametime: {:?}", delta));
                        });

                    ui.show_demo_window(&mut demo_open);
                });
                manager_with_loop.reqwest_redraws();
            }
            Event::RedrawRequested(window_id) => {
                if let Some(draw_data) = platform.draw_data(&mut imgui, *window_id) {
                    let viewport = manager_with_loop
                        .viewport_mut(*window_id)
                        .expect("Expect viewport");
                    viewport.on_draw(&mut renderer, draw_data);
                }
            }
            _ => {}
        }
        platform.handle_event(imgui.io_mut(), &mut manager, &event);
    });
}
