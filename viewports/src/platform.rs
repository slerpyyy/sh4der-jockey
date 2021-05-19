use winit::{
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    window::WindowId,
};

use imgui::{sys as imgui_sys, BackendFlags, Context, ImString, Io, Key, Ui};
use imgui_sys::{ImGuiPlatformIO, ImGuiViewport};
use std::{
    cmp::Ordering,
    rc::Rc,
    time::{Duration, Instant},
};

mod callbacks;
mod proxy;
use proxy::{Cache, Proxy, SharedProxy};

/// winit backend platform state
#[derive(Debug)]
pub struct Platform {
    main_view: WindowId,
    proxy: SharedProxy,
    last_frame: Instant,
}

impl Platform {
    /// Initializes a winit platform instance and configures imgui.
    ///
    /// This function configures imgui-rs in the following ways:
    ///
    /// * backend flags are updated
    /// * keys are configured
    /// * platform name is set
    pub fn init<V: crate::Viewport>(imgui: &mut Context, main_view: &V) -> Platform {
        imgui.set_platform_name(Some(ImString::from(format!(
            "imgui-winit-support-viewports {}",
            env!("CARGO_PKG_VERSION")
        ))));

        let io = imgui.io_mut();
        let has_viewports = unsafe {
            BackendFlags::from_bits_unchecked(
                imgui_sys::ImGuiBackendFlags_PlatformHasViewports
                    | imgui_sys::ImGuiBackendFlags_RendererHasViewports,
            )
        };
        io.backend_flags.insert(has_viewports);
        io.backend_flags.insert(BackendFlags::HAS_MOUSE_CURSORS);
        //io.backend_flags.insert(BackendFlags::HAS_SET_MOUSE_POS);

        io[Key::Tab] = VirtualKeyCode::Tab as _;
        io[Key::LeftArrow] = VirtualKeyCode::Left as _;
        io[Key::RightArrow] = VirtualKeyCode::Right as _;
        io[Key::UpArrow] = VirtualKeyCode::Up as _;
        io[Key::DownArrow] = VirtualKeyCode::Down as _;
        io[Key::PageUp] = VirtualKeyCode::PageUp as _;
        io[Key::PageDown] = VirtualKeyCode::PageDown as _;
        io[Key::Home] = VirtualKeyCode::Home as _;
        io[Key::End] = VirtualKeyCode::End as _;
        io[Key::Insert] = VirtualKeyCode::Insert as _;
        io[Key::Delete] = VirtualKeyCode::Delete as _;
        io[Key::Backspace] = VirtualKeyCode::Back as _;
        io[Key::Space] = VirtualKeyCode::Space as _;
        io[Key::Enter] = VirtualKeyCode::Return as _;
        io[Key::Escape] = VirtualKeyCode::Escape as _;
        io[Key::KeyPadEnter] = VirtualKeyCode::NumpadEnter as _;
        io[Key::A] = VirtualKeyCode::A as _;
        io[Key::C] = VirtualKeyCode::C as _;
        io[Key::V] = VirtualKeyCode::V as _;
        io[Key::X] = VirtualKeyCode::X as _;
        io[Key::Y] = VirtualKeyCode::Y as _;
        io[Key::Z] = VirtualKeyCode::Z as _;

        io.display_framebuffer_scale = [1.0, 1.0];
        {
            let size = main_view.window().inner_size();
            io.display_size = [size.width as f32, size.height as f32];
        }
        //io.display_framebuffer_scale = [hidpi_factor as f32, hidpi_factor as f32];
        //let logical_size = window.inner_size().to_logical(hidpi_factor);
        //let logical_size = self.scale_size_from_winit(window, logical_size);
        //io.display_size = [logical_size.width as f32, logical_size.height as f32];

        //let cache = HashMap::new();
        //cache.insert(main_view, Cache::default());

        let proxy = Proxy::shared();
        let main_view = main_view.window().id();
        let main_view_key = proxy.borrow_mut().use_window(main_view);

        unsafe {
            use imgui::internal::RawCast;
            io.raw_mut().BackendPlatformUserData = Rc::into_raw(Rc::clone(&proxy)) as _;
        }

        let platform_io = imgui.platform_io();
        callbacks::register_platform_callbacks(platform_io);

        unsafe {
            (*platform_io.MainViewport).PlatformUserData = main_view_key as _;
        }

        /*assert_eq!(std::mem::size_of::<WindowId>(), std::mem::size_of::<usize>());
        unsafe {
            (*platform_io.MainViewport).PlatformHandle = std::mem::transmute(main_view);
            //use imgui::internal::RawCast;
            //imgui.io_mut().raw_mut().BackendPlatformUserData = Rc::into_raw(Rc::clone(&proxy)) as _;
        }*/

        let last_frame = Instant::now();

        Platform {
            //hidpi_mode: ActiveHiDpiMode::Default,
            //hidpi_factor: 1.0,
            //cursor_cache: None,
            main_view,
            proxy,
            last_frame,
        }
    }

    pub fn handle_event<T, M: crate::Manager>(
        &mut self,
        io: &mut Io,
        window_manager: &mut M,
        event: &Event<T>,
    ) {
        match *event {
            Event::WindowEvent {
                window_id,
                ref event,
            } => {
                let viewport = window_manager.viewport_mut(window_id);
                let main_view = self.main_view;
                if let Some(viewport) = viewport {
                    let mut proxy = self.proxy.borrow_mut();
                    let cache = proxy.expect_cache_by_wid(window_id).1;
                    Self::handle_window_event(io, viewport, cache, event);
                    if window_id == main_view {
                        Self::handle_main_view_event(io, viewport, cache, event);
                    }
                }
                self.handle_global_event(io, event);
            }
            _ => (),
        }
    }

    fn handle_main_view_event<V: crate::Viewport>(
        io: &mut Io,
        _viewport: &mut V,
        _cache: &mut Cache,
        event: &WindowEvent,
    ) {
        match *event {
            WindowEvent::Resized(physical_size) => {
                io.display_size = [physical_size.width as f32, physical_size.height as f32];
            }
            _ => {}
        }
    }

    fn handle_window_event<V: crate::Viewport>(
        io: &mut Io,
        viewport: &mut V,
        cache: &mut Cache,
        event: &WindowEvent,
    ) {
        match *event {
            WindowEvent::ScaleFactorChanged {
                scale_factor: _, ..
            } => {
                /*let hidpi_factor = match self.hidpi_mode {
                    ActiveHiDpiMode::Default => scale_factor,
                    ActiveHiDpiMode::Rounded => scale_factor.round(),
                    _ => return,
                };
                // Mouse position needs to be changed while we still have both the old and the new
                // values
                if io.mouse_pos[0].is_finite() && io.mouse_pos[1].is_finite() {
                    io.mouse_pos = [
                        io.mouse_pos[0] * (hidpi_factor / self.hidpi_factor) as f32,
                        io.mouse_pos[1] * (hidpi_factor / self.hidpi_factor) as f32,
                    ];
                }
                self.hidpi_factor = hidpi_factor;
                io.display_framebuffer_scale = [hidpi_factor as f32, hidpi_factor as f32];
                // Window size might change too if we are using DPI rounding
                let logical_size = window.inner_size().to_logical(scale_factor);
                let logical_size = self.scale_size_from_winit(window, logical_size);
                io.display_size = [logical_size.width as f32, logical_size.height as f32];*/
            }
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                io.keys_down[key as usize] = true;
            }
            WindowEvent::ReceivedCharacter(ch) => {
                // Exclude the backspace key ('\u{7f}'). Otherwise we will insert this char and then
                // delete it.
                if ch != '\u{7f}' {
                    io.add_input_character(ch)
                }
            }
            WindowEvent::Focused(focus) => {
                cache.focus = focus;
            }
            WindowEvent::Moved(pos) => {
                #[cfg(windows)]
                {
                    if pos == [-32000, -32000].into() {
                        cache.minimized = true;
                    } else {
                        cache.minimized = false;
                    }
                }
                if !cache.minimized {
                    cache.set_pos(pos);
                }
            }
            WindowEvent::Resized(size) => {
                if size == [0, 0].into() {
                    cache.minimized = true;
                } else {
                    cache.minimized = false;
                    cache.set_size(size);
                    viewport.on_resize();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                /*
                let position = position.to_logical(window.scale_factor());
                let position = self.scale_pos_from_winit(window, position);
                io.mouse_pos = [position.x as f32, position.y as f32];
                */
                let position = position.cast::<f32>();
                let winpos = viewport.window().outer_position().unwrap().cast::<f32>();
                io.mouse_pos = [position.x + winpos.x, position.y + winpos.y];
            }
            WindowEvent::CursorLeft { .. } => {
                io.mouse_pos = [f32::MIN, f32::MIN];
            }
            WindowEvent::MouseWheel {
                delta,
                phase: TouchPhase::Moved,
                ..
            } => match delta {
                MouseScrollDelta::LineDelta(h, v) => {
                    io.mouse_wheel_h = h;
                    io.mouse_wheel = v;
                }
                MouseScrollDelta::PixelDelta(pos) => {
                    //let pos = pos.to_logical::<f64>(self.hidpi_factor);
                    match pos.x.partial_cmp(&0.0) {
                        Some(Ordering::Greater) => io.mouse_wheel_h += 1.0,
                        Some(Ordering::Less) => io.mouse_wheel_h -= 1.0,
                        _ => (),
                    }
                    match pos.y.partial_cmp(&0.0) {
                        Some(Ordering::Greater) => io.mouse_wheel += 1.0,
                        Some(Ordering::Less) => io.mouse_wheel -= 1.0,
                        _ => (),
                    }
                }
            },
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                match button {
                    MouseButton::Left => io.mouse_down[0] = pressed,
                    MouseButton::Right => io.mouse_down[1] = pressed,
                    MouseButton::Middle => io.mouse_down[2] = pressed,
                    MouseButton::Other(idx @ 0..=4) => io.mouse_down[idx as usize] = pressed,
                    _ => (),
                }
            }
            _ => (),
        }
    }
    fn handle_global_event(&mut self, io: &mut Io, event: &WindowEvent) {
        match *event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Released,
                        ..
                    },
                ..
            } => {
                io.keys_down[key as usize] = false;

                // This is a bit redundant here, but we'll leave it in. The OS occasionally
                // fails to send modifiers keys, but it doesn't seem to send false-positives,
                // so double checking isn't terrible in case some system *doesn't* send
                // device events sometimes.
                match key {
                    VirtualKeyCode::LShift | VirtualKeyCode::RShift => io.key_shift = false,
                    VirtualKeyCode::LControl | VirtualKeyCode::RControl => io.key_ctrl = false,
                    VirtualKeyCode::LAlt | VirtualKeyCode::RAlt => io.key_alt = false,
                    VirtualKeyCode::LWin | VirtualKeyCode::RWin => io.key_super = false,
                    _ => (),
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                io.key_shift = modifiers.shift();
                io.key_ctrl = modifiers.ctrl();
                io.key_alt = modifiers.alt();
                io.key_super = modifiers.logo();
            }
            _ => {}
        }
    }
    pub fn frame<
        T,
        M: crate::Manager,
        F: FnOnce(&Ui, Duration),
        S: super::WindowSpawner<M::Viewport>,
    >(
        &mut self,
        imgui: &mut Context,
        manager: &mut crate::WithLoop<M, T, S>,
        frame: F,
    ) {
        update_monitors(manager, imgui.platform_io());

        let now = Instant::now();
        let delta_s = now - self.last_frame;
        imgui.io_mut().update_delta_time(delta_s);
        self.last_frame = now;

        self.proxy.borrow_mut().update(manager);

        let ui = imgui.frame();
        frame(&ui, delta_s);
        let _ = ui.render();

        self.proxy.borrow_mut().update(manager);

        /*if last_cursor != Some(ui.mouse_cursor()) {
            last_cursor = Some(ui.mouse_cursor());
            platform.prepare_render(&ui, active.expect_native_window(first_id));
        }*/
        imgui.update_platform_windows();
        self.proxy.borrow_mut().update(manager);
    }
    pub fn draw_data<'a>(
        &self,
        imgui: &'a mut imgui::Context,
        wid: WindowId,
    ) -> Option<&'a imgui::DrawData> {
        use imgui::internal::RawCast;
        let platform = imgui.platform_io();
        let mut proxy = self.proxy.borrow_mut();
        // first frame there can be no window
        let (&search_key, cache) = proxy.cache_by_wid(wid)?;
        if cache.minimized {
            return None;
        }

        unsafe {
            let viewports: &[*mut ImGuiViewport] =
                std::slice::from_raw_parts(platform.Viewports.Data, platform.Viewports.Size as _);
            for vp in viewports.iter().filter_map(|vp| vp.as_ref()) {
                if vp.PlatformUserData.is_null() {
                    continue;
                }
                let key: proxy::Key = std::mem::transmute(vp.PlatformUserData);
                if key != search_key {
                    continue;
                }
                let draw_data = RawCast::from_raw(vp.DrawData.as_ref()?);
                return Some(draw_data);
            }
        }
        None
    }
    pub fn last_frame(&self) -> Instant {
        self.last_frame
    }
}

fn update_monitors<M, T, S>(with_loop: &crate::WithLoop<M, T, S>, platform: &mut ImGuiPlatformIO) {
    use imgui_sys::{ImGuiPlatformMonitor, ImVec2};
    let mut monitors = if platform.Monitors.Data.is_null() {
        Vec::with_capacity(with_loop.event_loop.available_monitors().size_hint().0)
    } else {
        use std::mem::replace;
        let raw = &mut platform.Monitors;
        let ptr = replace(&mut raw.Data, std::ptr::null_mut());
        let length = replace(&mut raw.Size, 0) as usize;
        let capacity = replace(&mut raw.Capacity, 0) as usize;
        assert!(length < 32);
        assert!(capacity <= length);
        unsafe { Vec::from_raw_parts(ptr, length, capacity) }
    };
    monitors.clear();
    monitors.extend(
        with_loop
            .event_loop
            .available_monitors()
            .take(32)
            .map(|monitor| {
                let pos = monitor.position();
                let posf = ImVec2 {
                    x: pos.x as _,
                    y: pos.y as _,
                };
                let size = monitor.size();
                let sizef = ImVec2 {
                    x: size.width as _,
                    y: size.height as _,
                };

                ImGuiPlatformMonitor {
                    MainPos: posf,
                    MainSize: sizef,
                    WorkPos: posf,
                    WorkSize: sizef,
                    DpiScale: monitor.scale_factor() as _,
                }
            }),
    );
    //let (ptr, length, capacity) = monitors.into_raw_parts();
    //use std::convert::TryInto;
    let (ptr, length, capacity) = (monitors.as_mut_ptr(), monitors.len(), monitors.capacity());
    std::mem::forget(monitors);
    let raw = &mut platform.Monitors;
    raw.Capacity = capacity as _;
    raw.Size = length as _;
    raw.Data = ptr;
}

unsafe trait HasPlatformIO {
    fn platform_io(&mut self) -> &mut ImGuiPlatformIO {
        unsafe {
            imgui_sys::igGetPlatformIO()
                .as_mut()
                .expect("ImGuiPlatformIO")
        }
    }
    fn update_platform_windows(&mut self) {
        unsafe {
            imgui_sys::igUpdatePlatformWindows();
        }
    }
}
unsafe impl HasPlatformIO for imgui::Context {}
