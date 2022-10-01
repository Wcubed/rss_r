use actix_web_lab::__reexports::futures_util::future;
use chrono::{Duration, Utc};
use feed_rs::model;
use reqwest::ClientBuilder;
use rss_com_lib::rss_feed::{FeedEntries, FeedEntry};
use rss_com_lib::Url;
use std::collections::{HashMap, HashSet};
use std::error::Error;

pub struct FeedRequester {
    reqwest_client: reqwest::Client,
}

impl Default for FeedRequester {
    fn default() -> Self {
        FeedRequester {
            reqwest_client: ClientBuilder::new()
                .build()
                .expect("Could not build reqwest client"),
        }
    }
}

impl FeedRequester {
    /// Downloads all the feeds concurrently.
    pub async fn request_feeds(
        &self,
        urls: &HashSet<Url>,
        timeout: core::time::Duration,
    ) -> HashMap<Url, Result<Feed, Box<dyn Error>>> {
        let results =
            future::join_all(urls.iter().map(|url| self.request_feed(url, timeout))).await;

        results.into_iter().collect()
    }

    pub async fn request_feed(
        &self,
        url: &Url,
        timeout: core::time::Duration,
    ) -> (Url, Result<Feed, Box<dyn Error>>) {
        (url.clone(), self.download_feed(url, timeout).await)
    }

    async fn download_feed(
        &self,
        url: &Url,
        timeout: core::time::Duration,
    ) -> Result<Feed, Box<dyn Error>> {
        // TODO (Wybe 2022-07-18): Sanitize url.
        let content = self
            .reqwest_client
            .get(url.clone_string())
            .timeout(timeout)
            .send()
            .await?
            .bytes()
            .await?;

        let raw_feed = feed_rs::parser::parse(&content[..])?;

        let entries = FeedEntries::new(
            raw_feed
                .entries
                .iter()
                .map(FeedEntry::from_raw_feed_entry)
                .collect(),
        );

        let feed = Feed {
            title: raw_feed.title.map(|text| text.content).unwrap_or_default(),
            entries,
        };

        Ok(feed)
    }
}

pub struct Feed {
    pub title: String,
    pub entries: FeedEntries,
}
