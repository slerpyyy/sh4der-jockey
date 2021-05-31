use super::Texture;
use std::{collections::HashMap, rc::Rc};

static mut CACHE_INTERNAL: Option<HashMap<String, CacheEntry>> = None;

#[derive(Debug)]
struct CacheEntry {
    tex: Rc<dyn Texture>,
}

impl CacheEntry {
    pub fn new(tex: Rc<dyn Texture>) -> Self {
        Self { tex }
    }
}

pub struct Cache;

impl Cache {
    pub fn init() {
        unsafe {
            if CACHE_INTERNAL.is_none() {
                CACHE_INTERNAL = Some(HashMap::new());
            }
        }
    }

    fn internal() -> &'static HashMap<String, CacheEntry> {
        Self::internal_mut()
    }

    fn internal_mut() -> &'static mut HashMap<String, CacheEntry> {
        #[cfg(debug_assertions)]
        if unsafe { CACHE_INTERNAL.is_none() } {
            panic!("Cache has not been initialized. Please call `Cache::init` first.")
        }

        unsafe { CACHE_INTERNAL.as_mut().unwrap() }
    }

    pub fn store(path: String, tex: Rc<dyn Texture>) {
        let entry = CacheEntry::new(tex);
        Self::internal_mut().insert(path, entry);
    }

    pub fn fetch(path: &str) -> Option<Rc<dyn Texture>> {
        Self::internal().get(path).map(|s| Rc::clone(&s.tex))
    }
}
