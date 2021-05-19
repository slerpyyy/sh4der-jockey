mod platform;
use bitflags::bitflags;
use imgui::sys as imgui_sys;
use std::ops::{Deref, DerefMut};
use winit::{
    event_loop::EventLoopWindowTarget,
    window::{Window, WindowBuilder, WindowId},
};

pub use platform::Platform;

#[cfg(feature = "wgpu-renderer")]
pub mod wgpu;

// reexport git-based forks
pub mod dependencies {
    pub use imgui;
    #[cfg(feature = "wgpu-renderer")]
    pub use imgui_wgpu;
}

pub trait Viewport {
    type Renderer;
    fn window(&self) -> &Window;
    fn on_resize(&mut self);
    fn on_draw(&mut self, renderer: &mut Self::Renderer, draw_data: &imgui::DrawData);
}

pub trait Manager: Sized {
    type Renderer;
    type Viewport: Viewport<Renderer = Self::Renderer>;

    fn viewport(&self, wid: WindowId) -> Option<&Self::Viewport>;
    fn viewport_mut(&mut self, wid: WindowId) -> Option<&mut Self::Viewport>;
    fn add_window(&mut self, window: Window) -> WindowId;
    fn destroy(&mut self, wid: WindowId);

    fn with_loop<'a, T: 'static>(
        &'a mut self,
        event_loop: &'a EventLoopWindowTarget<T>,
    ) -> WithLoop<'a, Self, T> {
        Self::with_spawner(self, event_loop, DefaultSpawner)
    }
    fn with_spawner<'a, T: 'static, S: WindowSpawner<Self::Viewport>>(
        &'a mut self,
        event_loop: &'a EventLoopWindowTarget<T>,
        spawner: S,
    ) -> WithLoop<'a, Self, T, S> {
        WithLoop {
            manager: self,
            event_loop,
            spawner,
        }
    }
}

pub struct WithLoop<'a, M, T: 'static, S = DefaultSpawner> {
    manager: &'a mut M,
    event_loop: &'a EventLoopWindowTarget<T>,
    spawner: S,
}

impl<'a, M: Manager, T, S: WindowSpawner<M::Viewport>> WithLoop<'a, M, T, S> {
    fn spawn_window(&mut self, flags: ViewportFlags) -> WindowId {
        let window = self.spawner.build_window(self.event_loop, flags);
        self.manager.add_window(window)
    }
}

impl<'a, M, T: 'static, S> Deref for WithLoop<'a, M, T, S> {
    type Target = M;
    fn deref(&self) -> &Self::Target {
        &self.manager
    }
}
impl<'a, M, T: 'static, S> DerefMut for WithLoop<'a, M, T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.manager
    }
}

pub trait WindowSpawner<V: Viewport> {
    fn build_window<T: 'static>(
        &mut self,
        event_loop: &EventLoopWindowTarget<T>,
        flags: ViewportFlags,
    ) -> Window;
    fn show_window(&mut self, viewport: &V);
}
pub struct DefaultSpawner;
impl<V: Viewport> WindowSpawner<V> for DefaultSpawner {
    fn build_window<T: 'static>(
        &mut self,
        event_loop: &EventLoopWindowTarget<T>,
        flags: ViewportFlags,
    ) -> Window {
        let decorations = !flags.contains(ViewportFlags::NO_DECORATIONS);
        WindowBuilder::new()
            .with_decorations(decorations)
            .build(event_loop)
            .unwrap()
    }
    fn show_window(&mut self, viewport: &V) {
        viewport.window().set_visible(true);
    }
}

//use imgui_sys::ImGuiWindowFlags;

bitflags! {
    #[repr(transparent)]
    pub struct ViewportFlags: u32 {
        const NO_DECORATIONS = imgui_sys::ImGuiViewportFlags_NoDecoration;

        const NO_TASK_BAR_ICON = imgui_sys::ImGuiViewportFlags_NoTaskBarIcon;

        const NO_FOCUS_ON_APPEARING = imgui_sys::ImGuiViewportFlags_NoFocusOnAppearing;

        const NO_FOCUS_ON_CLICK = imgui_sys::ImGuiViewportFlags_NoFocusOnClick;

        const NO_INPUTS = imgui_sys::ImGuiViewportFlags_NoInputs;

        const NO_RENDERER_CLEAR = imgui_sys::ImGuiViewportFlags_NoRendererClear;

        const TOPMOST = imgui_sys::ImGuiViewportFlags_TopMost;

        const MINIMIZED = imgui_sys::ImGuiViewportFlags_Minimized;

        const NO_AUTO_MERGE = imgui_sys::ImGuiViewportFlags_NoAutoMerge;

        const CAN_HOST_OTHER_WINDOWS = imgui_sys::ImGuiViewportFlags_CanHostOtherWindows;
    }
}
