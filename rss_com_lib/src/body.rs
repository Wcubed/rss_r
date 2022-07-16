use serde::{Deserialize, Serialize};

/// Request format for `/api/does_feed_exist`
#[derive(Serialize, Deserialize)]
pub struct DoesFeedExistRequest {
    pub url: String,
}

/// Response format for `/api/does_feed_exist`
#[derive(Serialize, Deserialize)]
pub struct DoesFeedExistResponse {
    pub requested_url: String,
    /// Either the title of the feed, or the error message if there is no feed.
    pub result: Result<String, String>,
}
