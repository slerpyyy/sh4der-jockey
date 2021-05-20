pub struct DockSpace {}

impl DockSpace {
    pub fn new() -> Self {
        unsafe {
            let flags = imgui::sys::ImGuiDockNodeFlags_None as i32;
            let viewport = imgui::sys::igGetMainViewport();
            let window_class = imgui::sys::ImGuiWindowClass_ImGuiWindowClass();

            imgui::sys::igDockSpaceOverViewport(viewport, flags, window_class);
        }

        Self {}
    }
}
