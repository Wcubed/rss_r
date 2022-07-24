use chrono::{DateTime, Duration, Utc};
use log::info;
use rss::Channel;
use std::collections::HashMap;
use std::error::Error;
use std::sync::RwLock;

/// Url -> rss channel.
/// TODO (Wybe 2022-07-18): Can we remember old entries that are no longer in the feed if there is a user that hasn't read them?
/// TODO (Wybe 2022-07-18): rss feeds sometimes have a `Cloud` entry that allows subscribing. How do we do this?
#[derive(Default)]
pub struct RssCache(RwLock<HashMap<String, CacheEntry>>);

impl RssCache {
    /// Returns the feed immediately if it is in the cache, and up-to-date.
    /// Does a request for the feed if it isn't in the cache.
    /// Returns an error if the feed could not be retrieved.
    ///
    /// Not efficient when iterating over all the feeds, unless all of the `get_feed` calls are
    /// awaited in parallel.
    /// TODO (Wybe 2022-07-18): Add an iter() method that does many parallel requests for all the feeds, and eagerly returns those that it can.
    pub async fn get_feed(&self, url: &str) -> Result<Channel, Box<dyn Error>> {
        let cached_channel = {
            // TODO (Wybe 2022-07-18): Make this configurable.
            let max_cache_age = Duration::hours(1);

            // Check the cache.
            let cache = self.0.read().unwrap();
            if let Some(feed) = cache.get(url) {
                let now = Utc::now();

                if now - feed.last_checked < max_cache_age {
                    Some(feed.channel.clone())
                } else {
                    // Cache entry too old, download again.
                    None
                }
            } else {
                None
            }
        };

        match cached_channel {
            Some(channel) => Ok(channel),
            None => {
                // Feed is not in cache, or is too old.
                let result = RssCache::download_feed(url).await;
                if let Ok(channel) = &result {
                    self.add_to_cache(url, channel.clone());
                }

                result
            }
        }
    }

    fn add_to_cache(&self, url: &str, channel: Channel) {
        let now = Utc::now();

        let mut cache = self.0.write().unwrap();
        cache.insert(
            url.to_string(),
            CacheEntry {
                channel,
                last_checked: now,
            },
        );
    }

    async fn download_feed(url: &str) -> Result<Channel, Box<dyn Error>> {
        // TODO (Wybe 2022-07-18): Sanitize url.
        let content = reqwest::get(url).await?.bytes().await?;
        let channel = Channel::read_from(&content[..])?;
        Ok(channel)
    }
}

pub struct CacheEntry {
    pub channel: Channel,
    pub last_checked: DateTime<Utc>,
}
