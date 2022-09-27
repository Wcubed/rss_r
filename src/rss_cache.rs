use actix_web_lab::__reexports::futures_util::{future, StreamExt};
use chrono::{DateTime, Duration, Utc};
use feed_rs::model;
use rss_com_lib::Url;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::RwLock;

/// Url -> Feed.
/// TODO (Wybe 2022-09-19): It would probably be better to remember feed entries in a central location, and only store per-user info for each of them in the user's locations.
/// TODO (Wybe 2022-07-18): rss feeds sometimes have a `Cloud` entry that allows subscribing. How do we do this?
#[derive(Default)]
pub struct RssCache(RwLock<HashMap<Url, CacheEntry>>);

impl RssCache {
    /// Gets all the listed feeds.
    /// Makes an http request for a feed if it isn't in the cache.
    /// All requests are made concurrently.
    pub async fn get_feeds(
        &self,
        urls: &HashSet<Url>,
    ) -> HashMap<Url, Result<model::Feed, Box<dyn Error>>> {
        let results = future::join_all(urls.iter().map(|url| self.get_feed(url))).await;

        results.into_iter().collect()
    }

    /// Returns the feed immediately if it is in the cache, and up-to-date.
    /// Does a request for the feed if it isn't in the cache.
    /// Returns an error if the feed could not be retrieved.
    ///
    /// Not efficient when iterating over all the feeds, unless all of the `get_feed` calls are
    /// awaited in parallel.
    pub async fn get_feed(&self, url: &Url) -> (Url, Result<model::Feed, Box<dyn Error>>) {
        let cached_feed = {
            // TODO (Wybe 2022-07-18): Make this configurable.
            let max_cache_age = Duration::hours(1);

            // Check the cache.
            let cache = self.0.read().unwrap();
            if let Some(entry) = cache.get(url) {
                let now = Utc::now();

                if now - entry.last_checked < max_cache_age {
                    Some(entry.feed.clone())
                } else {
                    // Cache entry too old, download again.
                    None
                }
            } else {
                None
            }
        };

        match cached_feed {
            Some(feed) => (url.clone(), Ok(feed)),
            None => {
                // Feed is not in cache, or is too old.
                let result = RssCache::download_feed(url).await;
                if let Ok(feed) = &result {
                    self.add_to_cache(url, feed.clone());
                }

                (url.clone(), result)
            }
        }
    }

    fn add_to_cache(&self, url: &Url, feed: model::Feed) {
        let now = Utc::now();

        let mut cache = self.0.write().unwrap();
        cache.insert(
            url.clone(),
            CacheEntry {
                feed,
                last_checked: now,
            },
        );
    }

    async fn download_feed(url: &Url) -> Result<model::Feed, Box<dyn Error>> {
        // TODO (Wybe 2022-07-18): Sanitize url.
        let content = reqwest::get(url.clone_string()).await?.bytes().await?;
        let feed = feed_rs::parser::parse(&content[..])?;
        Ok(feed)
    }
}

pub struct CacheEntry {
    pub feed: model::Feed,
    pub last_checked: DateTime<Utc>,
}
