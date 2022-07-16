use crate::users::UserId;
use crate::{Authenticated, SaveInRonFile};
use actix_web::{post, web, HttpResponse, Responder};
use log::info;
use rss::Channel;
use rss_com_lib::body::{
    AddFeedRequest, IsUrlAnRssFeedRequest, IsUrlAnRssFeedResponse, ListFeedsResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;

///TODO (Wybe 2022-07-16): Change to rwlock.
#[derive(Default, Serialize, Deserialize, Debug)]
pub struct RssCollections(Mutex<HashMap<UserId, RssCollection>>);

impl SaveInRonFile for RssCollections {
    const FILE_NAME: &'static str = "rss_collections.ron";
}

impl std::ops::Deref for RssCollections {
    type Target = Mutex<HashMap<UserId, RssCollection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RssCollections {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Hashmap<url, ()>
/// TODO (Wybe 2022-07-16): Make the key an actual Url type. Watch out that that might not be json serializable.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RssCollection(HashMap<String, ()>);

impl std::ops::Deref for RssCollection {
    type Target = HashMap<String, ()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RssCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Returns a list of all feeds in a users collection.
#[post("/list_feeds")]
pub async fn list_feeds(
    auth: Authenticated,
    collections: web::Data<RssCollections>,
) -> impl Responder {
    let collections = collections.lock().unwrap();

    let feeds = if let Some(collection) = collections.get(auth.user_id()) {
        collection.keys().cloned().collect()
    } else {
        Vec::new()
    };

    HttpResponse::Ok().json(ListFeedsResponse { feeds })
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
        let mut collections = collections.lock().unwrap();
        let collection = if let Some(collection) = collections.get_mut(auth.user_id()) {
            collection
        } else {
            collections.insert(*auth.user_id(), RssCollection::default());
            collections.get_mut(auth.user_id()).unwrap()
        };

        // TODO (Wybe 2022-07-16): Check if the feed is already in the collection.
        collection.insert(request.url.to_string(), ());
    }

    collections.save();
    HttpResponse::Ok().finish()
}

/// Checks a given rss feed for existence.
/// Sends back the title of the feed if it exists.
/// TODO (Wybe 2022-07-14): Sanitize url?
/// TODO (Wybe 2022-07-14): Can we do Rust object notation, instead of parsing from Json?
/// TODO (Wybe 2022-07-16): Cache feed, so we can immediately serve it after it has been added.
#[post("/is_url_an_rss_feed")]
pub async fn is_url_an_rss_feed(
    request: web::Json<IsUrlAnRssFeedRequest>,
    auth: Authenticated,
) -> impl Responder {
    info!(
        "User `{}` tests url `{}` for existence of an rss feed",
        auth.user_name(),
        request.url,
    );

    let result = match download_feed(&request.url).await {
        Ok(channel) => Ok(channel.title),
        Err(err) => Err(err.to_string()),
    };

    HttpResponse::Ok().json(IsUrlAnRssFeedResponse {
        requested_url: request.url.to_string(),
        result,
    })
}

async fn download_feed(url: &str) -> Result<Channel, Box<dyn Error>> {
    let content = reqwest::get(url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
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
    fn test_serialize_ron_rss_collections_with_one_collecxtion() {
        let collections = RssCollections::default();

        // Note that this scope block is necessary, otherwise we still have the lock
        // while the `to_string_pretty` also wants the lock. Which would deadlock the
        // thread.
        {
            let mut lock = collections.lock().unwrap();

            let collection = RssCollection::default();
            lock.insert(UserId(0), collection);
        }

        assert!(to_string_pretty(&collections, PrettyConfig::default()).is_ok());
    }
}
