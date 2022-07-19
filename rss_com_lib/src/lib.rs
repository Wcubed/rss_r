#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod body;

use rss::Item;
use serde::{Deserialize, Serialize};

pub const USER_ID_HEADER: &str = "user_id";
pub const PASSWORD_HEADER: &str = "user_pass";

#[derive(Serialize, Deserialize, Clone)]
pub struct FeedEntry {
    /// TODO (Wybe 2022-07-18): Pass a guid, whether it was read or not, and the date.
    pub title: String,
    /// Link to the original content.
    pub link: Option<String>,
}

impl FeedEntry {
    pub fn from_rss_item(item: &Item) -> Self {
        Self {
            title: match &item.title {
                Some(title) => title.clone(),
                None => "No title".to_string(),
            },
            link: item.link.clone(),
        }
    }
}
