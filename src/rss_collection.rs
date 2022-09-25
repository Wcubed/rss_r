use crate::users::UserId;
use crate::{Authenticated, RssCache, SaveInRonFile};
use actix_web::{post, web, HttpResponse, Responder};
use log::{info, warn};
use rss_com_lib::message_body::{
    AddFeedRequest, GetFeedEntriesRequest, GetFeedEntriesResponse, IsUrlAnRssFeedRequest,
    IsUrlAnRssFeedResponse, ListFeedsResponse,
};
use rss_com_lib::rss_feed::{EntryKey, FeedEntries, FeedEntry, FeedInfo};
use rss_com_lib::Url;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Iter;
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};
use std::sync::RwLock;

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct RssCollections(RwLock<HashMap<UserId, RssCollection>>);

impl SaveInRonFile for RssCollections {
    const FILE_NAME: &'static str = "rss_collections.ron";
}

impl std::ops::Deref for RssCollections {
    type Target = RwLock<HashMap<UserId, RssCollection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RssCollections {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RssCollection(HashMap<Url, RssFeed>);

impl std::ops::Deref for RssCollection {
    type Target = HashMap<Url, RssFeed>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RssCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RssFeed {
    info: FeedInfo,
    entries: FeedEntries,
}

impl RssFeed {
    pub fn new(name: String) -> Self {
        RssFeed {
            info: FeedInfo { name },
            entries: FeedEntries::default(),
        }
    }

    /// Checks if any of the given entries are new, and updates the feed with them.
    /// Leaves any existing entries as-is.
    pub fn update_entries(&mut self, entries: FeedEntries) {
        // TODO (Wybe 2022-09-25): Write a test for this.
        for (key, entry) in entries.into_iter() {
            self.entries.entry(key).or_insert(entry);
        }
    }
}

/// Returns a list of all feeds in a users collection.
#[post("/list_feeds")]
pub async fn list_feeds(
    auth: Authenticated,
    collections: web::Data<RssCollections>,
) -> impl Responder {
    let collections = collections.read().unwrap();

    let feeds = if let Some(collection) = collections.get(auth.user_id()) {
        collection
            .iter()
            .map(|(key, feed)| (key.clone(), feed.info.clone()))
            .collect()
    } else {
        HashMap::new()
    };

    HttpResponse::Ok().json(ListFeedsResponse { feeds })
}

#[post("/get_feed_entries")]
pub async fn get_feed_entries(
    request: web::Json<GetFeedEntriesRequest>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
    cache: web::Data<RssCache>,
) -> impl Responder {
    let result = {
        let mut collections = collections.write().unwrap();

        // TODO (Wybe 2022-07-18): Do this in a way that does not block on `collections` while the potential request
        //      for a feed update is being awaited.
        if let Some(collection) = collections.get_mut(auth.user_id()) {
            let results_map =
                get_feeds_from_cache_or_update(auth, collection, cache, &request.feeds).await;

            if results_map.is_empty() {
                // An empty map means the user requested only feeds that they don't have in the collection.
                HttpResponse::Unauthorized().finish()
            } else {
                HttpResponse::Ok().json(GetFeedEntriesResponse {
                    results: results_map,
                })
            }
        } else {
            HttpResponse::Forbidden().finish()
        }
    };

    // The user's collection might have gotten updated.
    // TODO (Wybe 2022-09-24): Save every x minutes if the collections have changed, instead of saving every time a request comes in?
    collections.save();

    result
}

/// Returns requested feeds, keyed by their url, or an error message
/// if that feed doesn't exist.
/// Makes any needed requests to each feed's original url concurrently.
/// TODO (Wybe 2022-07-19): Don't block on collection?
/// TODO (Wybe 2022-07-19): Rename this
async fn get_feeds_from_cache_or_update(
    auth: Authenticated,
    collection: &mut RssCollection,
    cache: web::Data<RssCache>,
    requests: &HashSet<Url>,
) -> HashMap<Url, Result<(FeedInfo, FeedEntries), String>> {
    let mut results = HashMap::new();

    for url in requests.iter() {
        let result = if collection.contains_key(url) {
            // TODO (Wybe 2022-07-19): Somehow await the get_feed_entries all at the same time? Instead of each one after the other.
            match cache.get_feed(url).await {
                Ok(channel) => {
                    // Update the user's collection and add it to the results.
                    let entries = FeedEntries::new(
                        channel.items.iter().map(FeedEntry::from_rss_item).collect(),
                    );

                    let feed = match collection.get_mut(url) {
                        None => {
                            collection.insert(url.clone(), RssFeed::new(channel.title.clone()));
                            collection.get_mut(url).unwrap()
                        }
                        Some(feed) => feed,
                    };

                    feed.update_entries(entries);
                    // TODO (Wybe 2022-09-20): Can we circumvent this clone? And take the reference instead? Or is that performance optimization not needed (probably not, given 1 user)?
                    Ok((feed.info.clone(), feed.entries.clone()))
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            warn!(
                "User `{}` made a request for a feed that is not in their collection: `{}`",
                auth.user_name(),
                url
            );
            Err("Feed is not in the collection".to_string())
        };

        results.insert(url.clone(), result);
    }

    results
}

/// Adds the given rss feed to the feed collection of the user.
/// TODO (Wybe 2022-07-16): Sanitize url?
/// TODO (Wybe 2022-07-16): Check if the feed actually exists.
/// TODO (Wybe 2022-07-16): Cache feed, so we can immediately serve it after it has been added.
#[post("/add_feed")]
pub async fn add_feed(
    request: web::Json<AddFeedRequest>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
) -> impl Responder {
    info!(
        "Adding feed for user `{}`: `{}`",
        auth.user_name(),
        request.url
    );

    {
        let mut collections = collections.write().unwrap();
        let collection = if let Some(collection) = collections.get_mut(auth.user_id()) {
            collection
        } else {
            collections.insert(*auth.user_id(), RssCollection::default());
            collections.get_mut(auth.user_id()).unwrap()
        };

        if !collection.contains_key(&request.url) {
            // This feed is new for the user.
            collection.insert(request.url.clone(), RssFeed::new(request.name.clone()));
        } else {
            info!(
                "User `{}` already had feed `{}` in their collection",
                auth.user_name(),
                request.url
            );
            // TODO (Wybe 2022-09-19): Return an error.
        }
    }

    collections.save();
    HttpResponse::Ok().finish()
}

/// Checks a given rss feed for existence.
/// Sends back the title of the feed if it exists.
/// TODO (Wybe 2022-07-14): Can we do Rust object notation, instead of parsing from Json?
#[post("/is_url_an_rss_feed")]
pub async fn is_url_an_rss_feed(
    request: web::Json<IsUrlAnRssFeedRequest>,
    auth: Authenticated,
    cache: web::Data<RssCache>,
) -> impl Responder {
    info!(
        "User `{}` tests url `{}` for existence of an rss feed",
        auth.user_name(),
        request.url,
    );

    let result = match cache.get_feed(&request.url).await {
        Ok(channel) => Ok(channel.title),
        Err(err) => Err(err.to_string()),
    };

    HttpResponse::Ok().json(IsUrlAnRssFeedResponse {
        requested_url: Url::new(request.url.to_string()),
        result,
    })
}

#[cfg(test)]
mod tests {
    use crate::rss_collection::{RssCollection, RssFeed};
    use crate::users::UserId;
    use crate::RssCollections;
    use pretty_assertions::assert_eq;
    use ron::ser::{to_string_pretty, PrettyConfig};
    use rss_com_lib::rss_feed::{EntryKey, FeedEntries, FeedEntry};
    use rss_com_lib::Url;
    use std::collections::HashMap;

    #[test]
    fn test_serialize_ron_rss_collections_empty() {
        let collections = RssCollections::default();

        assert!(to_string_pretty(&collections, PrettyConfig::default()).is_ok());
    }

    #[test]
    fn test_serialize_ron_rss_collections_with_one_collection() {
        let collections = RssCollections::default();

        // Note that this scope block is necessary, otherwise we still have the lock
        // while the `to_string_pretty` also wants the lock. Which would deadlock the
        // thread.
        {
            let mut lock = collections.write().unwrap();

            let collection = RssCollection::default();
            lock.insert(UserId(0), collection);
        }

        assert!(to_string_pretty(&collections, PrettyConfig::default()).is_ok());
    }

    #[test]
    fn test_updating_feed_leaves_existing_entries_intact() {
        // Given
        let mut feed = RssFeed::new("Feed".to_string());

        let entry_1 = FeedEntry {
            title: "Title".to_string(),
            link: Some(Url::new("a link".to_string())),
            pub_date: Default::default(),
        };
        let key_1 = EntryKey::from_entry(&entry_1);

        feed.entries.insert(key_1.clone(), entry_1.clone());

        let entry_2 = FeedEntry {
            title: "Title".to_string(),
            link: Some(Url::new("another_link".to_string())),
            pub_date: Default::default(),
        };
        let key_2 = EntryKey::from_entry(&entry_2);

        // Double-check that the entry keys will be the same.
        // With the same keys, entry 2 will overwrite entry 1 if it is inserted in the
        // feed. We want to check that this insertion doesn't happen.
        assert_eq!(key_1, key_2);

        let mut update_entries = FeedEntries::default();
        update_entries.insert(key_2, entry_2);

        // When
        feed.update_entries(update_entries);

        // Then
        // entry_1 is still in the feed.
        let expected_map = HashMap::<EntryKey, FeedEntry>::from([(key_1, entry_1)]);
        assert_eq!(feed.entries.inner(), expected_map);
    }
}
