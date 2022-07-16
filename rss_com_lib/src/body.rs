use serde::{Deserialize, Serialize};

/// Request format for `/api/is_url_an_rss_feed`
#[derive(Serialize, Deserialize)]
pub struct IsUrlAnRssFeedRequest {
    pub url: String,
}

/// Response format for `/api/is_url_an_rss_feed`
#[derive(Serialize, Deserialize)]
pub struct IsUrlAnRssFeedResponse {
    pub requested_url: String,
    /// Either the title of the feed, or the error message if there is no feed.
    pub result: Result<String, String>,
}

/// Request format for `/api/add_feed`
/// The response is an Ok with an empty body, if the adding worked.
#[derive(Serialize, Deserialize)]
pub struct AddFeedRequest {
    pub url: String,
    pub name: String,
}

/// Response for `/api/list_feeds`
#[derive(Serialize, Deserialize)]
pub struct ListFeedsResponse {
    /// List of url's and names
    /// TODO (Wybe 2022-07-16): Make actually url types, and not flat strings?
    /// TODO (Wybe 2022-07-16): Use references, so we don't copy strigns on every request?
    pub feeds: Vec<(String, String)>,
}
