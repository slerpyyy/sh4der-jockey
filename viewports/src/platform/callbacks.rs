use super::proxy::{Key, Proxy, SharedProxy};
use crate::ViewportFlags;
use imgui::sys as imgui_sys;
use imgui_sys::{ImGuiPlatformIO, ImGuiViewport, ImVec2};
use std::rc::Rc;

pub(super) trait Callbacks {
    fn create_window(&mut self, flags: ViewportFlags) -> Key;
    fn destroy_window(&mut self, key: Key);
    fn show_window(&mut self, key: Key);
    fn set_position(&mut self, key: Key, pos: ImVec2);
    fn set_size(&mut self, key: Key, size: ImVec2);
    fn set_focus(&mut self, key: Key);
    fn get_position(&self, key: Key) -> ImVec2;
    fn get_size(&self, key: Key) -> ImVec2;
    fn get_focus(&self, key: Key) -> bool;
    fn get_minimized(&self, key: Key) -> bool;
    fn set_title(&mut self, key: Key, title: String);
}

unsafe fn from_vp<R: 'static, F: FnOnce(&mut Proxy, &mut Key) -> R>(
    vp: *mut ImGuiViewport,
    callback: F,
) -> R {
    let vp = &mut (*vp);
    let ptr = (*imgui_sys::igGetIO()).BackendPlatformUserData;
    assert_eq!(ptr.is_null(), false);
    let proxy: SharedProxy = Rc::from_raw(ptr as _);
    let ret = {
        let mut guard = proxy.borrow_mut();
        let key: &mut Key = std::mem::transmute(&mut vp.PlatformUserData);
        callback(&mut *guard, key)
    };
    std::mem::forget(proxy);
    ret
}

pub fn register_platform_callbacks(platform: &mut ImGuiPlatformIO) {
    unsafe extern "C" fn create_window(vp: *mut ImGuiViewport) {
        from_vp(vp, |proxy, key| {
            assert_eq!(*key, 0);
            let flags = (*vp).Flags as u32;
            *key = proxy.create_window(ViewportFlags::from_bits_unchecked(flags));
            //dbg!(key);
            //dbg!((*vp).PlatformUserData);
        });
    }
    platform.Platform_CreateWindow = Some(create_window);

    unsafe extern "C" fn destroy_window(vp: *mut ImGuiViewport) {
        from_vp(vp, |proxy, key| {
            proxy.destroy_window(*key);
            *key = 0;
        });
    }
    platform.Platform_DestroyWindow = Some(destroy_window);

    unsafe extern "C" fn show_window(vp: *mut ImGuiViewport) {
        from_vp(vp, |proxy, key| {
            proxy.show_window(*key);
        });
    }
    platform.Platform_ShowWindow = Some(show_window);

    unsafe extern "C" fn set_window_pos(vp: *mut ImGuiViewport, pos: ImVec2) {
        from_vp(vp, |proxy, key| {
            proxy.set_position(*key, pos);
        });
    }
    platform.Platform_SetWindowPos = Some(set_window_pos);

    unsafe extern "C" fn get_window_pos(vp: *mut ImGuiViewport, pos: *mut ImVec2) {
        /*if (*vp).PlatformUserData as usize > 1 {
            println!("get_window_pos!!!");
        }*/
        *pos = from_vp(vp, |proxy, key| proxy.get_position(*key));
    }
    unsafe {
        ImGuiPlatformIO_Set_Platform_GetWindowPos(platform, get_window_pos);
    }

    unsafe extern "C" fn set_window_size(vp: *mut ImGuiViewport, size: ImVec2) {
        from_vp(vp, |proxy, key| {
            proxy.set_size(*key, size);
        })
    }
    platform.Platform_SetWindowSize = Some(set_window_size);

    unsafe extern "C" fn get_window_size(vp: *mut ImGuiViewport, size: *mut ImVec2) {
        *size = from_vp(vp, |proxy, key| proxy.get_size(*key));
    }
    unsafe {
        ImGuiPlatformIO_Set_Platform_GetWindowSize(platform, get_window_size);
    }

    unsafe extern "C" fn set_window_focus(vp: *mut ImGuiViewport) {
        from_vp(vp, |proxy, key| {
            proxy.set_focus(*key);
        });
    }
    platform.Platform_SetWindowFocus = Some(set_window_focus);

    unsafe extern "C" fn get_window_focus(vp: *mut ImGuiViewport) -> bool {
        from_vp(vp, |proxy, key| proxy.get_focus(*key))
    }
    platform.Platform_GetWindowFocus = Some(get_window_focus);

    unsafe extern "C" fn get_window_minimized(vp: *mut ImGuiViewport) -> bool {
        from_vp(vp, |proxy, key| proxy.get_minimized(*key))
    }
    platform.Platform_GetWindowMinimized = Some(get_window_minimized);

    unsafe extern "C" fn set_window_title(
        vp: *mut ImGuiViewport,
        str: *const ::std::os::raw::c_char,
    ) {
        let title = std::ffi::CStr::from_ptr(str).to_bytes();
        from_vp(vp, |proxy, key| {
            proxy.set_title(*key, std::str::from_utf8(title).unwrap().to_owned());
        });
    }
    platform.Platform_SetWindowTitle = Some(set_window_title);
}

type PlatformUserCallback = unsafe extern "C" fn(*mut ImGuiViewport, *mut ImVec2);
extern "C" {
    //void ImGuiPlatformIO_Set_Platform_GetWindowPos(ImGuiPlatformIO* platform_io, void(*user_callback)(ImGuiViewport* vp, ImVec2* out_pos))
    fn ImGuiPlatformIO_Set_Platform_GetWindowPos(
        platform_io: &mut ImGuiPlatformIO,
        user_callback: PlatformUserCallback,
    );
    //void ImGuiPlatformIO_Set_Platform_GetWindowSize(ImGuiPlatformIO* platform_io, void(*user_callback)(ImGuiViewport* vp, ImVec2* out_size))
    fn ImGuiPlatformIO_Set_Platform_GetWindowSize(
        platform_io: &mut ImGuiPlatformIO,
        user_callback: PlatformUserCallback,
    );
}
