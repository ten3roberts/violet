use std::sync::Arc;

use parking_lot::Mutex;

use crate::{declare_atom, stored::Handle};

pub struct Clipboard {
    inner: ClipboardInner,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: ClipboardInner::new(),
        }
    }

    pub async fn get_text(&self) -> Option<String> {
        self.inner.get_text().await
    }

    pub async fn set_text(&self, text: String) {
        self.inner.set_text(text).await
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct ClipboardInner {
    clipboard: Mutex<arboard::Clipboard>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ClipboardInner {
    pub fn new() -> Self {
        Self {
            clipboard: Mutex::new(arboard::Clipboard::new().unwrap()),
        }
    }

    pub async fn get_text(&self) -> Option<String> {
        self.clipboard.lock().get_text().ok()
    }

    pub async fn set_text(&self, text: String) {
        self.clipboard.lock().set_text(text).ok();
    }
}

#[cfg(target_arch = "wasm32")]
struct ClipboardInner {
    clipboard: Option<web_sys::Clipboard>,
}

#[cfg(target_arch = "wasm32")]
impl ClipboardInner {
    pub fn new() -> Self {
        Self {
            clipboard: web_sys::window().map(|v| v.navigator().clipboard()),
        }
    }

    pub async fn get_text(&self) -> Option<String> {
        Some(
            wasm_bindgen_futures::JsFuture::from(self.clipboard.as_ref()?.read_text())
                .await
                .ok()?
                .as_string()
                .expect("Result should be a string"),
        )
    }

    pub async fn set_text(&self, text: String) {
        if let Some(clipboard) = &self.clipboard {
            wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&text))
                .await
                .ok();
        }
    }
}

declare_atom! {
    pub clipboard: Handle<Arc<Clipboard>>,
}
