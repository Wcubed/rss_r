use crate::users::UserId;
use crate::{Authenticated, RssCache, SaveInRonFile};
use actix_web::{post, web, HttpResponse, Responder};
use log::info;
use rss_com_lib::body::{
    AddFeedRequest, GetFeedRequest, GetFeedResponse, IsUrlAnRssFeedRequest, IsUrlAnRssFeedResponse,
    ListFeedsResponse,
};
use rss_com_lib::{FeedEntry, FeedSelection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

///TODO (Wybe 2022-07-16): Change to rwlock.
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

/// Hashmap<url, feed>
/// TODO (Wybe 2022-07-16): Make the key an actual Url type. Watch out that that might not be json serializable.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RssCollection(HashMap<String, RssFeed>);

impl std::ops::Deref for RssCollection {
    type Target = HashMap<String, RssFeed>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RssCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RssFeed {
    name: String,
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
            .map(|(key, feed)| (key.clone(), feed.name.clone()))
            .collect()
    } else {
        Vec::new()
    };

    HttpResponse::Ok().json(ListFeedsResponse { feeds })
}

#[post("/get_feed")]
pub async fn get_feed(
    request: web::Json<GetFeedRequest>,
    auth: Authenticated,
    collections: web::Data<RssCollections>,
    cache: web::Data<RssCache>,
) -> impl Responder {
    let feed_request = &request.feed;

    let collections = collections.read().unwrap();
    // TODO (Wybe 2022-07-18): Do this in a way that does not block on `collections` while the potential request
    //      for a feed update is being awaited.
    if let Some(collection) = collections.get(auth.user_id()) {
        match feed_request {
            FeedSelection::All => {
                // TODO (Wybe 2022-07-18): Implement
                HttpResponse::Forbidden().finish()
            }
            FeedSelection::Feed(url) => {
                if collection.contains_key(url) {
                    let result = match cache.get_feed(url).await {
                        Ok(channel) => {
                            Ok(channel.items.iter().map(FeedEntry::from_rss_item).collect())
                        }
                        Err(e) => Err(e.to_string()),
                    };

                    let mut results_map = HashMap::new();
                    results_map.insert(url.clone(), result);

                    HttpResponse::Ok().json(GetFeedResponse {
                        requested_selection: feed_request.clone(),
                        results: results_map,
                    })
                } else {
                    HttpResponse::Forbidden().finish()
                }
            }
        }
    } else {
        HttpResponse::Forbidden().finish()
    }
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

        // TODO (Wybe 2022-07-16): Check if the feed is already in the collection.
        collection.insert(
            request.url.to_string(),
            RssFeed {
                name: request.name.clone(),
            },
        );
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
        requested_url: request.url.to_string(),
        result,
    })
}

#[cfg(test)]
mod tests {
    use crate::rss_collection::RssCollection;
    use crate::users::UserId;
    use crate::RssCollections;
    use ron::ser::{to_string_pretty, PrettyConfig};

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
}
