use crate::users::UserId;
use crate::{full_error_to_string, Authenticated, FeedRequester, SaveInRonFile};
use actix_web::{post, web, HttpResponse, Responder};
use log::info;
use rss_com_lib::message_body::{
    AddFeedRequest, AdditionalAction, ComFeedEntry, EntryTypeFilter, FeedsFilter, FeedsRequest,
    FeedsResponse, IsUrlAnRssFeedRequest, IsUrlAnRssFeedResponse, SetEntryReadRequestAndResponse,
    SetFeedInfoRequestAndResponse,
};
use rss_com_lib::rss_feed::{FeedEntries, FeedInfo};
use rss_com_lib::Url;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::RwLock;
use std::time::Duration;

const NEW_FEED_REQUEST_TIMEOUT: Duration = core::time::Duration::from_secs(10);

#[derive(Default, Serialize, Deserialize, Debug)]
pub struct RssCollections(RwLock<HashMap<UserId, RssCollection>>);

impl Hash for RssCollections {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let collections = self.read().unwrap();
        for (user, collection) in collections.iter() {
            user.hash(state);
            collection.hash(state);
        }
    }
}

/// TODO (Wybe 2022-09-25): Implement that this is saved every minute or so if it has changed. But not every time a request comes through.
///   Also, it should be saved when the server is stopped, for example by pressing Ctrl+C.
impl SaveInRonFile for RssCollections {
    const FILE_NAME: &'static str = "collections.ron";
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

impl RssCollection {
    /// Returns the entries, and how many there were in total.
    fn get_sorted_com_entries_with_filter(
        &self,
        amount: usize,
        feed_filter: FeedsFilter,
        entry_filter: EntryTypeFilter,
    ) -> (Vec<ComFeedEntry>, usize) {
        let mut entries: Vec<ComFeedEntry> = match feed_filter {
            FeedsFilter::All => self
                .iter()
                .flat_map(|(url, feed)| {
                    feed.entries
                        .iter()
                        .filter(|(_, entry)| entry_filter.apply(entry))
                        .map(|(key, entry)| ComFeedEntry::new(url.clone(), key.clone(), entry))
                })
                .collect(),
            FeedsFilter::Tag(tag) => self
                .iter()
                .filter(|(_, feed)| feed.info.tags.contains(&tag))
                .flat_map(|(url, feed)| {
                    feed.entries
                        .iter()
                        .filter(|(_, entry)| entry_filter.apply(entry))
                        .map(|(key, entry)| ComFeedEntry::new(url.clone(), key.clone(), entry))
                })
                .collect(),
            FeedsFilter::Single(url) => {
                if let Some(feed) = self.get(&url) {
                    feed.entries
                        .iter()
                        .filter(|(_, entry)| entry_filter.apply(entry))
                        .map(|(key, entry)| ComFeedEntry::new(url.clone(), key.clone(), entry))
                        .collect()
                } else {
                    vec![]
                }
            }
        };

        entries.sort();
        let total = entries.len();

        entries.truncate(amount);
        (entries, total)
    }
}

impl Hash for RssCollection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The hashmap always returns the values in the same order, unless it has been changed.
        // Which is exactly what we want, because the hash is used for change detection.
        for (url, feed) in self.iter() {
            url.hash(state);
            feed.hash(state);
        }
    }
}

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

/// Represents a single rss feed.
///
/// Implements [Default] so that adding new entries won't break the loading of old files.
#[derive(Serialize, Deserialize, Debug, Default, Hash)]
#[serde(default)]
pub struct RssFeed {
    info: FeedInfo,
    entries: FeedEntries,
}

impl RssFeed {
    pub fn new(info: FeedInfo, entries: FeedEntries) -> Self {
        RssFeed { info, entries }
    }

    /// Checks if any of the given entries are new, and updates the feed with them.
    /// Leaves any existing entries as-is.
    pub fn update_entries(&mut self, maybe_entries: Result<FeedEntries, String>) {
        match maybe_entries {
            Ok(entries) => {
                for (key, entry) in entries.into_iter() {
                    self.entries.entry(key).or_insert(entry);
                }

                self.info.last_update_result = Ok(());
            }
            Err(error) => {
                self.info.last_update_result = Err(error);
            }
        }
    }
}

#[post("/feeds")]
pub async fn get_feeds(
    request: web::Json<FeedsRequest>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
    requester: web::Data<FeedRequester>,
) -> impl Responder {
    let result = {
        let feeds_info = match request.additional_action {
            AdditionalAction::None => None,
            AdditionalAction::IncludeFeedsInfo => {
                let collections = collections.read().unwrap();
                collections.get(auth.user_id()).map(|collection| {
                    collection
                        .iter()
                        .map(|(key, feed)| (key.clone(), feed.info.clone()))
                        .collect()
                })
            }
            AdditionalAction::UpdateFeeds => {
                // Update all url's
                // We collect the urls to be updated separately from the update:
                // Because according to clippy, it is not a good idea to hold a mutex lock across an `await`.
                let maybe_urls = {
                    let collections = collections.read().unwrap();

                    info!("User {} requested refresh of feeds.", auth.user_name());

                    collections
                        .get(auth.user_id())
                        .map(|collection| collection.iter().map(|(url, _)| url.clone()).collect())
                };

                if let Some(urls) = maybe_urls {
                    let update_timeout = core::time::Duration::from_secs(5);

                    // This is the call that performs the actual updates.
                    // TODO (2024-09-03): On the raspberry pi there are too many requests that go wrong, that go ok the next time I try.
                    //                    This does not happen when I test this locally on my laptop. Then only the feeds that don't exist get a red question mark.
                    //                    What are the errors that happen, and why?
                    let mut feeds = requester.request_feeds(&urls, update_timeout).await;

                    let mut collections = collections.write().unwrap();
                    if let Some(collection) = collections.get_mut(auth.user_id()) {
                        for url in &urls {
                            if let Some(feed) = collection.get_mut(url) {
                                // Feed exists in the users collection.
                                if let Some(maybe_feed_update) = feeds.remove(url) {
                                    let maybe_entries = maybe_feed_update
                                        .map(|feed| feed.entries)
                                        .map_err(|error| full_error_to_string(&error));
                                    feed.update_entries(maybe_entries);
                                } else {
                                    // Feed is in the users collection, but the update request did not return a result.
                                    feed.update_entries(Err("Feed update was requested, but the function did not return anything.".to_string()));
                                }
                            }
                        }

                        Some(
                            collection
                                .iter()
                                .map(|(key, feed)| (key.clone(), feed.info.clone()))
                                .collect(),
                        )
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        let collections = collections.read().unwrap();

        if let Some(collection) = collections.get(auth.user_id()) {
            let (entries, total) = collection.get_sorted_com_entries_with_filter(
                request.amount,
                request.filter.clone(),
                request.entry_filter,
            );

            HttpResponse::Ok().json(FeedsResponse {
                feed_entries: entries,
                total_available: total,
                feeds_info,
            })
        } else {
            HttpResponse::Forbidden().finish()
        }
    };

    result
}

/// Adds the given rss feed to the feed collection of the user.
/// TODO (Wybe 2022-07-16): Sanitize url?
#[post("/add_feed")]
pub async fn add_feed(
    request: web::Json<AddFeedRequest>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
    requester: web::Data<FeedRequester>,
) -> impl Responder {
    info!(
        "Adding feed for user `{}`: `{}`",
        auth.user_name(),
        request.url
    );

    {
        // TODO (2024-08-21): Don't hold the collections mutex accross the await point.
        let mut collections = collections.write().unwrap();
        let collection = if let Some(collection) = collections.get_mut(auth.user_id()) {
            collection
        } else {
            collections.insert(*auth.user_id(), RssCollection::default());
            collections.get_mut(auth.user_id()).unwrap()
        };

        if !collection.contains_key(&request.url) {
            // This feed is new for the user.
            if let (_, Ok(new_feed)) = requester
                .request_feed(&request.url, NEW_FEED_REQUEST_TIMEOUT)
                .await
            {
                let info = FeedInfo {
                    name: new_feed.title,
                    tags: request.tags.clone(),
                    last_update_result: Ok(()),
                };

                collection.insert(request.url.clone(), RssFeed::new(info, new_feed.entries));
            } else {
                // TODO (Wybe 2022-10-01): Return an error.
            }
        } else {
            info!(
                "User `{}` already had feed `{}` in their collection",
                auth.user_name(),
                request.url
            );
            // TODO (Wybe 2022-09-19): Return an error.
        }
    }

    HttpResponse::Ok().finish()
}

/// Checks a given rss feed for existence.
/// Sends back the title of the feed if it exists.
/// TODO (Wybe 2022-07-14): Can we do Rust object notation, instead of parsing from Json?
/// TODO (Wybe 2022-09-27): Also allow linking the main page of a comic, and figuring out by any rss/feed href where the feed is located.
#[post("/is_url_an_rss_feed")]
pub async fn is_url_an_rss_feed(
    request: web::Json<IsUrlAnRssFeedRequest>,
    auth: Authenticated,
    requester: web::Data<FeedRequester>,
) -> impl Responder {
    info!(
        "User `{}` tests url `{}` for existence of an rss feed",
        auth.user_name(),
        request.url,
    );

    let (_, maybe_feed) = requester
        .request_feed(&request.url, NEW_FEED_REQUEST_TIMEOUT)
        .await;
    let result = match maybe_feed {
        Ok(feed) => Ok(feed.title),
        Err(err) => Err(err.to_string()),
    };

    HttpResponse::Ok().json(IsUrlAnRssFeedResponse {
        requested_url: Url::new(request.url.to_string()),
        result,
    })
}

#[post("/set_entry_read")]
pub async fn set_entry_read(
    request: web::Json<SetEntryReadRequestAndResponse>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
) -> impl Responder {
    {
        let mut collections = collections.write().unwrap();
        if let Some(collection) = collections.get_mut(auth.user_id()) {
            if let Some(feed) = collection.get_mut(&request.feed_url) {
                if let Some(entry) = feed.entries.get_mut(&request.entry_key) {
                    entry.read = request.read;
                } else {
                    // Entry does not exist in this feed.
                    return HttpResponse::Unauthorized().finish();
                }
            } else {
                // Feed does not exist for this user.
                return HttpResponse::Unauthorized().finish();
            }
        } else {
            // A collection does not exist for this user.
            return HttpResponse::Unauthorized().finish();
        };
    }

    // Send the request straight back to the client, so it doesn't need to remember all the
    // things it has requested from the server.
    HttpResponse::Ok().json(request.into_inner())
}

#[post("/set_feed_info")]
pub async fn set_feed_info(
    request: web::Json<SetFeedInfoRequestAndResponse>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
) -> impl Responder {
    {
        let mut collections = collections.write().unwrap();
        if let Some(collection) = collections.get_mut(auth.user_id()) {
            if let Some(feed) = collection.get_mut(&request.feed_url) {
                feed.info = request.info.clone();
            } else {
                // Feed does not exist for this user.
                return HttpResponse::Unauthorized().finish();
            }
        } else {
            // A collection does not exist for this user.
            return HttpResponse::Unauthorized().finish();
        };
    }

    // Send the request straight back to the client, so it doesn't need to remember all the
    // things it has requested from the server.
    HttpResponse::Ok().json(request.into_inner())
}

#[cfg(test)]
mod tests {
    use crate::rss_collection::{RssCollection, RssFeed};
    use crate::users::UserId;
    use crate::RssCollections;
    use pretty_assertions::assert_eq;
    use ron::ser::{to_string_pretty, PrettyConfig};
    use rss_com_lib::rss_feed::{EntryKey, FeedEntries, FeedEntry, FeedInfo};
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
        let mut feed = RssFeed::new(
            FeedInfo {
                name: "Test".to_string(),
                tags: Default::default(),
                last_update_result: Ok(()),
            },
            Default::default(),
        );

        let entry_1 = FeedEntry {
            title: "Title".to_string(),
            link: Some(Url::new("same link".to_string())),
            pub_date: Default::default(),
            read: false,
        };
        let key_1 = EntryKey::from_entry(&entry_1);

        feed.entries.insert(key_1.clone(), entry_1.clone());

        let entry_2 = FeedEntry {
            title: "Title".to_string(),
            link: Some(Url::new("same link".to_string())),
            pub_date: Default::default(),
            read: true,
        };
        let key_2 = EntryKey::from_entry(&entry_2);

        // Double-check that the entry keys will be the same.
        // With the same keys, entry 2 will overwrite entry 1 if it is inserted in the
        // feed. We want to check that this insertion doesn't happen.
        assert_eq!(key_1, key_2);

        let mut update_entries = FeedEntries::default();
        update_entries.insert(key_2, entry_2);

        // When
        feed.update_entries(Ok(update_entries));

        // Then
        // entry_1 is still in the feed.
        let expected_map = HashMap::<EntryKey, FeedEntry>::from([(key_1, entry_1)]);
        assert_eq!(feed.entries.inner(), expected_map);
    }
}
