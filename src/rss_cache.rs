use rss::Channel;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard};

/// Url -> rss channel.
/// TODO (Wybe 2022-07-18): Somehow make entries in the cache out of date if they are older than a certain amount
///                         and some way to force refresh?
#[derive(Default)]
pub struct RssCache(RwLock<HashMap<String, Channel>>);

impl RssCache {
    pub fn add_to_cache(&self, url: String, channel: Channel) {
        let mut cache = self.0.write().unwrap();

        cache.insert(url, channel);
    }

    pub fn read(&self) -> RwLockReadGuard<'_, HashMap<String, Channel>> {
        self.0.read().unwrap()
    }
}
