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
use log::Level;

#[cfg(target_arch = "wasm32")]
use eframe::wasm_bindgen::{self, prelude::*};
use egui::{Align2, Vec2};

/// All the popups should be aligned to this location.
const POPUP_ALIGN: Align2 = Align2::CENTER_TOP;
/// All the popups should be offset from [POPUP_ALIGN] by this much.
const POPUP_OFFSET: Vec2 = Vec2::new(0., 40.0);

/// This is the entry-point for all the web-assembly.
/// This is called once from the HTML.
/// It loads the app, installs some callbacks, then returns.
/// You can add more callbacks like this if you want to call in to your code.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) {
    // Make sure panics are logged using `console.error`.
    console_error_panic_hook::set_once();

    console_log::init_with_level(Level::Debug).unwrap();

    // Redirect tracing to console.log and friends:
    tracing_wasm::set_as_global_default();

    let web_options = eframe::WebOptions::default();
    eframe::start_web(
        canvas_id,
        web_options,
        Box::new(|cc| Box::new(RssApp::new(cc))),
    )
    .expect("Failed to start eframe");
}
