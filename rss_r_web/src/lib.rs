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

use egui::{Align2, Vec2};

/// All the popups should be aligned to this location.
const POPUP_ALIGN: Align2 = Align2::CENTER_TOP;
/// All the popups should be offset from [POPUP_ALIGN] by this much.
const POPUP_OFFSET: Vec2 = Vec2::new(0., 40.0);
