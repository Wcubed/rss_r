use crate::rss_feed::{EntryKey, FeedEntry, FeedInfo};
use crate::Url;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::default;

/// Request format for `/api/is_url_an_rss_feed`
#[derive(Serialize, Deserialize, Debug)]
pub struct IsUrlAnRssFeedRequest {
    pub url: Url,
}

/// Response format for `/api/is_url_an_rss_feed`
#[derive(Serialize, Deserialize, Debug)]
pub struct IsUrlAnRssFeedResponse {
    pub requested_url: Url,
    /// Name of the feed, or the error message if there is no feed.
    pub result: Result<String, String>,
}

/// Request format for `/api/add_feed`
/// The response is an Ok with an empty body, if the adding worked.
#[derive(Serialize, Deserialize, Debug)]
pub struct AddFeedRequest {
    pub url: Url,
    pub info: FeedInfo,
}

/// Request for `/api/feeds`
#[derive(Serialize, Deserialize, Debug)]
pub struct FeedsRequest {
    /// What feeds to return.
    pub filter: FeedsFilter,
    pub entry_filter: EntryTypeFilter,
    /// How many entries to return.
    pub amount: usize,
    pub additional_action: AdditionalAction,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub enum FeedsFilter {
    #[default]
    All,
    Tag(String),
    Single(Url),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum EntryTypeFilter {
    All,
    Unread,
}

impl EntryTypeFilter {
    pub fn apply(&self, entry: &FeedEntry) -> bool {
        match self {
            EntryTypeFilter::All => true,
            EntryTypeFilter::Unread => !entry.read,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AdditionalAction {
    None,
    /// Send along an update of all the feeds info.
    IncludeFeedsInfo,
    /// Update all the feeds, and send along an update of the feeds info.
    /// A request with this might take a while.
    UpdateFeeds,
}

/// Response for `/api/feeds`
#[derive(Serialize, Deserialize, Debug)]
pub struct FeedsResponse {
    /// Requested feed entries, ordered by time. Contains maximum [`FeedsRequest`] `.amount` entries.
    pub feed_entries: Vec<ComFeedEntry>,
    /// How many items were available for the given request.
    pub total_available: usize,
    /// If the request included [`AdditionalAction::IncludeFeedsInfo`] or [`AdditionalAction::UpdateFeeds`],
    /// this will be filled in. Otherwise it will be [`None`].
    pub feeds_info: Option<HashMap<Url, FeedInfo>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ComFeedEntry {
    /// Reference key to this specific entry.
    pub key: EntryKey,
    /// The feed this entry belongs to.
    pub feed_url: Url,
    pub title: String,
    /// Link to the original content.
    pub link: Option<Url>,
    /// If an rss feed includes an entry with no date, it will get a default date in the past.
    pub pub_date: DateTime<Utc>,
    pub read: bool,
}

impl ComFeedEntry {
    pub fn new(feed_url: Url, key: EntryKey, entry: &FeedEntry) -> Self {
        Self {
            key,
            feed_url,
            title: entry.title.clone(),
            link: entry.link.clone(),
            pub_date: entry.pub_date,
            read: entry.read,
        }
    }
}

impl PartialOrd for ComFeedEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComFeedEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Normally one would compare self to other.
        // However, the FeedEntries should be sorted with newest first,
        // so for the date we compare the other with self.
        let mut ord = other.pub_date.cmp(&self.pub_date);

        if ord != Ordering::Equal {
            return ord;
        }

        ord = self.title.cmp(&other.title);
        if ord != Ordering::Equal {
            return ord;
        }

        ord = self.link.cmp(&other.link);
        if ord != Ordering::Equal {
            return ord;
        }

        ord = self.read.cmp(&other.read);
        if ord != Ordering::Equal {
            return ord;
        }

        ord = self.key.cmp(&other.key);
        if ord != Ordering::Equal {
            return ord;
        }

        ord
    }
}

/// Request and response for `/api/set_entry_read`
/// The server sends the request straight back, so the client doesn't have to remember what
/// it requested from the server, and can simply "copy the server's notes".
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetEntryReadRequestAndResponse {
    pub feed_url: Url,
    pub entry_key: EntryKey,
    pub read: bool,
}

/// Request and response for `/api/set_feed_info`
/// The server sends the request straight back, so the client doesn't have to remember what
/// it requested from the server, and can simply "copy the server's notes".
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SetFeedInfoRequestAndResponse {
    pub feed_url: Url,
    pub info: FeedInfo,
}
