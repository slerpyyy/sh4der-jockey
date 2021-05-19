use imgui::sys::ImVec2;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::WindowId,
};

use crate::{Manager, Viewport, ViewportFlags, WindowSpawner, WithLoop};

pub(super) type Key = usize;
pub(super) type SharedProxy = Rc<RefCell<Proxy>>;

#[derive(Debug)]
pub struct Cache {
    pub(super) wid: WindowId,
    pub(super) minimized: bool,
    pub(super) focus: bool,
    pub(super) size: Option<ImVec2>,
    pub(super) pos: Option<ImVec2>,
}
impl Cache {
    fn new(wid: WindowId) -> Self {
        Self {
            wid,
            minimized: false,
            focus: true,
            size: None,
            pos: None,
        }
    }
    pub(super) fn set_size(&mut self, size: PhysicalSize<u32>) {
        self.size = Some(ImVec2 {
            x: size.width as _,
            y: size.height as _,
        });
    }
    pub(super) fn set_pos(&mut self, pos: PhysicalPosition<i32>) {
        self.pos = Some(ImVec2 {
            x: pos.x as _,
            y: pos.y as _,
        });
    }
}

#[derive(Debug)]
struct Command {
    key: Key,
    kind: Kind,
}
#[derive(Debug)]
enum Kind {
    CreateWindow { flags: ViewportFlags },
    DestroyWindow,
    ShowWindow,
    SetPos(ImVec2),
    SetSize(ImVec2),
    SetFocus,
    SetTitle(String),
}

#[derive(Debug)]
pub(super) struct Proxy {
    caches: HashMap<Key, Cache>,
    commands: Vec<Command>,
    next_id: Key,
}

impl Proxy {
    pub(super) fn new() -> Self {
        Self {
            caches: HashMap::new(),
            commands: vec![],
            next_id: 1,
        }
    }
    pub(super) fn shared() -> SharedProxy {
        Rc::new(RefCell::new(Self::new()))
    }
    pub(super) fn use_window(&mut self, wid: WindowId) -> Key {
        let cache = Cache::new(wid);
        let key = self.next_key();
        self.caches.insert(key, cache);
        key
    }
    pub(super) fn update<M: Manager, T, S: WindowSpawner<M::Viewport>>(
        &mut self,
        manager: &mut WithLoop<'_, M, T, S>,
    ) {
        /*if !self.commands.is_empty() {
            dbg!(&self.commands);
        }*/
        for Command { key, kind } in self.commands.drain(..) {
            match &kind {
                Kind::CreateWindow { flags } => {
                    let wid = manager.spawn_window(*flags);
                    let cache = Cache::new(wid);
                    self.caches.insert(key, cache);
                }
                Kind::DestroyWindow => {
                    let wid = self.caches.remove(&key).unwrap().wid;
                    manager.destroy(wid);
                }
                _ => {
                    let wid = self.caches.get(&key).unwrap().wid;
                    let viewport = manager.manager.viewport_mut(wid).expect("Expect viewport");
                    match kind {
                        Kind::CreateWindow { .. } | Kind::DestroyWindow => unreachable!(),
                        Kind::ShowWindow => {
                            manager.spawner.show_window(viewport);
                        }
                        Kind::SetPos(pos) => {
                            let pos = winit::dpi::PhysicalPosition {
                                x: pos.x.round() as i32,
                                y: pos.y.round() as i32,
                            };
                            viewport.window().set_outer_position(pos);
                        }
                        Kind::SetSize(size) => {
                            let size = winit::dpi::PhysicalSize {
                                width: size.x.round() as u32,
                                height: size.y.round() as u32,
                            };
                            viewport.window().set_inner_size(size);
                            viewport.on_resize();
                        }
                        Kind::SetFocus => {
                            //unimplemented!();
                        }
                        Kind::SetTitle(title) => viewport.window().set_title(&title),
                    }
                }
            }
        }
        for (_key, cache) in &mut self.caches {
            let wid = cache.wid;
            let viewport = manager.viewport_mut(wid).expect("Expect viewport");
            let window = viewport.window();
            if !cache.minimized {
                cache.set_size(window.inner_size());
                cache.set_pos(window.outer_position().unwrap());
            }
        }
    }
    fn next_key(&mut self) -> Key {
        let key = self.next_id;
        self.next_id += 1;
        key
    }
    /*pub fn draw_data<F>(
        &self,
        manager: &mut Manager,
        imgui: &mut imgui::Context,
        mut callback: F,
    ) where
        F: FnMut(&mut NativeWindow, &imgui::DrawData),
    {
        use imgui::internal::RawCast;
        let platform = imgui.platform_mut();
        let windows = &mut manager.windows;
        unsafe {
            let viewports =
                std::slice::from_raw_parts(platform.Viewports.Data, platform.Viewports.Size as _);
            for vp in viewports.iter() {
                if vp.is_null() {
                    continue;
                }
                let vp = &(**vp);
                if vp.DrawData.is_null() || vp.PlatformUserData.is_null() {
                    continue;
                }
                let key: Key = std::mem::transmute(vp.PlatformUserData);
                let cache = self.windows.get(&key).unwrap();
                let window = windows.get_mut(&cache.wid).unwrap();
                let draw_data = RawCast::from_raw(&*vp.DrawData);
                callback(window, draw_data);
            }
        }
    }*/

    #[track_caller]
    fn expect_cache(&self, key: Key) -> &Cache {
        self.caches.get(&key).expect("Expected cache!")
    }
    fn cache(&self, key: Key) -> Option<&Cache> {
        self.caches.get(&key)
    }
    fn cache_mut(&mut self, key: Key) -> Option<&mut Cache> {
        self.caches.get_mut(&key)
    }
    #[track_caller]
    pub(super) fn expect_cache_by_wid(&mut self, wid: WindowId) -> (&Key, &mut Cache) {
        self.caches
            .iter_mut()
            .find(|(_, cache)| cache.wid == wid)
            .expect("Expected cache!")
    }
    pub(super) fn cache_by_wid(&mut self, wid: WindowId) -> Option<(&Key, &mut Cache)> {
        self.caches.iter_mut().find(|(_, cache)| cache.wid == wid)
    }
}

impl super::callbacks::Callbacks for Proxy {
    fn create_window(&mut self, flags: ViewportFlags) -> Key {
        let key = self.next_key();
        self.commands.push(Command {
            key,
            kind: Kind::CreateWindow { flags },
        });
        key
    }
    fn destroy_window(&mut self, key: Key) {
        self.commands.push(Command {
            key,
            kind: Kind::DestroyWindow,
        });
    }
    fn show_window(&mut self, key: Key) {
        self.commands.push(Command {
            key,
            kind: Kind::ShowWindow,
        });
    }
    fn set_position(&mut self, key: Key, pos: ImVec2) {
        if let Some(cache) = self.cache_mut(key) {
            cache.pos = None;
        }
        self.commands.push(Command {
            key,
            kind: Kind::SetPos(pos),
        });
    }
    fn set_size(&mut self, key: Key, size: ImVec2) {
        if let Some(cache) = self.cache_mut(key) {
            cache.size = None;
        }
        self.commands.push(Command {
            key,
            kind: Kind::SetSize(size),
        });
    }
    fn set_focus(&mut self, key: Key) {
        self.commands.push(Command {
            key,
            kind: Kind::SetFocus,
        });
    }
    fn get_position(&self, key: Key) -> ImVec2 {
        self.expect_cache(key).pos.expect("Expect cached position")
    }
    fn get_size(&self, key: Key) -> ImVec2 {
        self.expect_cache(key).size.expect("Expect cached size")
    }
    fn get_focus(&self, key: Key) -> bool {
        // cache can be none for the first frame
        self.cache(key).map(|cache| cache.focus).unwrap_or(false)
    }
    fn get_minimized(&self, key: Key) -> bool {
        self.expect_cache(key).minimized
    }
    fn set_title(&mut self, key: Key, title: String) {
        self.commands.push(Command {
            key,
            kind: Kind::SetTitle(title),
        });
    }
}
