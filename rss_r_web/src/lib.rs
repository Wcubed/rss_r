#![deny(unsafe_code)]
#![warn(rust_2018_idioms, clippy::all)]

mod add_feed_popup;
mod app;
mod edit_feed_popup;
mod hyperlink;
mod login;
mod requests;
mod rss_collection;

pub use app::RssApp;
#[cfg(target_arch = "wasm32")]
use log::Level;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
use egui::{Align2, Vec2};

/// All the popups should be aligned to this location.
const POPUP_ALIGN: Align2 = Align2::CENTER_TOP;
/// All the popups should be offset from [POPUP_ALIGN] by this much.
const POPUP_OFFSET: Vec2 = Vec2::new(0., 40.0);

/// Your handle to the web app from JavaScript.
#[derive(Clone)]
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WebHandle {
    #[allow(clippy::new_without_default)]
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Make sure panics are logged using `console.error`.
        console_error_panic_hook::set_once();

        // Redirect [`log`] message to `console.log` and friends:
        console_log::init_with_level(Level::Debug).unwrap();

        // Redirect tracing to console.log and friends:
        tracing_wasm::set_as_global_default();

        Self {
            runner: eframe::WebRunner::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn start(&self, canvas_id: &str) -> Result<(), wasm_bindgen::JsValue> {
        let web_options = eframe::WebOptions::default();
        self.runner
            .start(
                canvas_id,
                web_options,
                Box::new(|cc| Box::new(RssApp::new(cc))),
            )
            .await
    }
}
