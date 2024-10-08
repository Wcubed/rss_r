#![deny(unsafe_code)]
#![warn(rust_2018_idioms, clippy::all)]

pub mod message_body;
pub mod rss_feed;

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Note: Http headers should not have underscores, proxies like nginx ignore them by default.
pub const USER_ID_HEADER: &str = "userid";
pub const PASSWORD_HEADER: &str = "userpass";

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Hash, Clone, Ord, PartialOrd)]
pub struct Url(String);

impl Url {
    pub fn new(url: String) -> Self {
        Self(url)
    }

    pub fn clone_string(&self) -> String {
        self.0.clone()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Display for Url {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
