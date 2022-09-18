#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod body;

use chrono::{DateTime, Utc};
use rss::Item;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const USER_ID_HEADER: &str = "user_id";
pub const PASSWORD_HEADER: &str = "user_pass";

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct FeedEntry {
    /// TODO (Wybe 2022-07-18): Pass a guid, whether it was read or not, and the date.
    pub title: String,
    /// Link to the original content.
    pub link: Option<String>,
    pub pub_date: Option<DateTime<Utc>>,
}

impl FeedEntry {
    pub fn from_rss_item(item: &Item) -> Self {
        let pub_date = item
            .pub_date
            .as_ref()
            .and_then(|ds| chrono::DateTime::parse_from_rfc2822(ds).ok())
            .map(|d| d.with_timezone(&Utc));

        Self {
            title: match &item.title {
                Some(title) => title.clone(),
                None => "No title".to_string(),
            },
            link: item.link.clone(),
            pub_date,
        }
    }
}

impl PartialOrd for FeedEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FeedEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Normally one would compare self to other.
        // However, the FeedEntries should be sorted with newest first,
        // so we compare the other with self.
        let mut ord = other.pub_date.cmp(&self.pub_date);

        if ord == Ordering::Equal {
            ord = self.title.cmp(&other.title);
        }

        if ord == Ordering::Equal {
            ord = self.link.cmp(&other.link);
        }

        ord
    }
}
