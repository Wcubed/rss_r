use crate::rss_feed::{EntryKey, FeedEntries, FeedInfo};
use crate::Url;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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

/// Response for `/api/list_feeds`
#[derive(Serialize, Deserialize, Debug)]
pub struct ListFeedsResponse {
    pub feeds: HashMap<Url, FeedInfo>,
}

/// Request for `/api/get_feed_entries`
#[derive(Serialize, Deserialize, Debug)]
pub struct GetFeedEntriesRequest {
    /// Urls of the feeds to retrieve.
    pub feeds: HashSet<Url>,
    /// Whether the server should first try to check for updates, before sending the feeds to the client.
    pub refresh: bool,
}

/// Response for `/api/get_feed_entries`
#[derive(Serialize, Deserialize, Debug)]
pub struct GetFeedEntriesResponse {
    /// Hashmap of
    /// <Feed url -> Feed>
    /// If the request contained any urls that are not in the user's collection, they will not
    /// be listed in the response.
    pub results: HashMap<Url, (FeedInfo, FeedEntries)>,
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
