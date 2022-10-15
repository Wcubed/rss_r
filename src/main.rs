#![deny(unsafe_code)]
#![warn(rust_2018_idioms, clippy::all)]

mod app_config;
mod auth;
mod auth_middleware;
mod error;
mod feed_requester;
mod persistence;
mod rss_collection;
mod users;

use crate::app_config::ApplicationConfig;
use crate::auth::{AuthData, AUTH_COOKIE_NAME};
use crate::auth_middleware::{AuthenticateMiddlewareFactory, Authenticated};
use crate::feed_requester::FeedRequester;
use crate::persistence::SaveInRonFile;
use crate::rss_collection::RssCollections;
use crate::users::UserInfo;
use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::SameSite;
use actix_web::middleware::Logger;
use actix_web::rt::{spawn, time};
use actix_web::web::Data;
use actix_web::{cookie, web, App, HttpServer};
use actix_web_lab::web::redirect;
use log::{info, warn, LevelFilter};
use simplelog::{
    format_description, ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode,
    WriteLogger,
};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::{create_dir_all, OpenOptions};
use std::hash::{Hash, Hasher};
use std::time::Duration;

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const LOGIN_DEADLINE: cookie::time::Duration = cookie::time::Duration::days(14);

/// How often the feed collections will be saved, if they have changed in the meantime.
const COLLECTIONS_SAVE_INTERVAL: Duration = Duration::from_secs(120);

/// How often we will update all of the user's feed collections in the background.
const FEED_UPDATE_INTERVAL: Duration = Duration::from_secs(3600 * 12);

/// TODO (Wybe 2022-07-10): Add some small banner that says this site uses cookies to authenticate? or is it not needed for authentication cookies.
/// TODO (Wybe 2022-07-12): Rss apparently sometimes allows getting push notifications, via a "Cloud" element in the feed. Is it worth it to implement this?
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    configure_logging();

    info!("Starting {} v{}", PACKAGE_NAME, VERSION);

    let app_config = ApplicationConfig::load_or_default();
    app_config.save();

    let auth_data = AuthData::load_or_default();
    auth_data.save();

    // TODO (Wybe 2022-07-12): Is it a problem to store the auth data as web data?
    //                         all services would be able to access it. But the services
    //                         are programmed by us, so is there a way for an outsider to exploit that?
    //                         It does increase the probability of mistakes to slip in i think.
    let web_auth_data = web::Data::new(auth_data);

    // TODO (Wybe 2022-07-16): Check whether all users that have a collection actually exist.
    let rss_collections = RssCollections::load_or_default();
    let web_rss_collections = web::Data::new(rss_collections);

    let binding_ip = app_config.binding_ip();
    info!(
        "Starting Http server at `{}`, with hostname `{}` and prefix `{}`",
        binding_ip, app_config.hostname, app_config.route_prefix
    );

    spawn_periodic_saving_task(web_rss_collections.clone(), COLLECTIONS_SAVE_INTERVAL);
    spawn_periodic_feed_update_task(web_rss_collections.clone(), FEED_UPDATE_INTERVAL);

    let collections_save_on_application_close = web_rss_collections.clone();

    HttpServer::new(move || {
        // TODO (Wybe 2022-07-11): Add an actual key?
        let identity_policy = CookieIdentityPolicy::new(&[0; 32])
            .name(AUTH_COOKIE_NAME)
            // Only transmit the cookie over secure connections.
            .secure(true)
            // Javascript is not allowed to see this cookie.
            .http_only(true)
            // No cross-site sending.
            .same_site(SameSite::Strict)
            // This is the maximum time that a login cookie is valid.
            // After this time the user has to log in again.
            .login_deadline(LOGIN_DEADLINE);

        App::new().wrap(Logger::default()).service(
            web::scope(&app_config.route_prefix)
                .service(redirect("/", "app/index.html"))
                .service(redirect("/app/", "index.html"))
                // This serves the static files of the rss_r_web webassembly application.
                .service(Files::new("/app", "resources/static"))
                .service(
                    web::scope("/api")
                        .app_data(web_auth_data.clone())
                        .app_data(web_rss_collections.clone())
                        .app_data(Data::new(FeedRequester::default()))
                        .wrap(AuthenticateMiddlewareFactory)
                        .wrap(IdentityService::new(identity_policy))
                        .service(auth::test_auth_cookie)
                        .service(auth::login)
                        .service(auth::logout)
                        .service(rss_collection::is_url_an_rss_feed)
                        .service(rss_collection::get_feed_entries)
                        .service(rss_collection::add_feed)
                        .service(rss_collection::list_feeds)
                        .service(rss_collection::set_entry_read)
                        .service(rss_collection::set_feed_info),
                ),
        )
    })
    .server_hostname(&app_config.hostname)
    .bind(binding_ip)?
    .run()
    .await?;

    // Make sure we don't loose anything that happened since the last save.
    collections_save_on_application_close.save();

    Ok(())
}

fn spawn_periodic_saving_task(collections: Data<RssCollections>, interval: Duration) {
    spawn(async move {
        let mut save_interval = time::interval(interval);

        let mut hasher = DefaultHasher::new();
        collections.hash(&mut hasher);
        let mut last_save_hash = hasher.finish();

        loop {
            save_interval.tick().await;

            let mut hasher = DefaultHasher::new();
            collections.hash(&mut hasher);
            let new_hash = hasher.finish();

            if new_hash != last_save_hash {
                // Collections have changed. Save them.
                collections.save();
                last_save_hash = new_hash;
            }
        }
    });
}

/// Will periodically update the feeds.
/// Will do the first update when this funcion is called.
fn spawn_periodic_feed_update_task(collections: Data<RssCollections>, interval: Duration) {
    spawn(async move {
        let mut update_interval = time::interval(interval);
        let feed_requester = FeedRequester::default();
        // The timeout for background updates can be a lot higher than when a user is waiting.
        let timeout = Duration::from_secs(20);

        loop {
            // The first time we get here, `tick` will immediately pass. This means we update
            // on the start of the program.
            update_interval.tick().await;

            update_all_collections(&collections, &feed_requester, timeout).await;
        }
    });
}

async fn update_all_collections(
    collections: &Data<RssCollections>,
    requester: &FeedRequester,
    timeout: Duration,
) {
    info!("Updating feeds in the background.");

    let mut feed_urls = HashSet::new();
    {
        let collections = collections.read().unwrap();

        for (_, collection) in collections.iter() {
            feed_urls.extend(collection.keys().cloned())
        }
    } // Lock on `RssCollections` is dropped here, so that it isn't held while the http requests are made (which can take quite a while).

    let feed_requests = requester.request_feeds(&feed_urls, timeout).await;

    {
        let mut collections = collections.write().unwrap();

        for (_, collection) in collections.iter_mut() {
            for (url, feed) in collection.iter_mut() {
                if let Some(Ok(update_feed)) = feed_requests.get(url) {
                    feed.update_entries(update_feed.entries.clone());
                }
            }
        }
    }

    info!("Done updating feeds in the background.")
}

fn configure_logging() {
    let log_dir = "log";

    // The logged time is by default in UTC.
    let config = ConfigBuilder::default()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        ))
        .set_thread_level(LevelFilter::Trace)
        .set_target_level(LevelFilter::Trace)
        .build();

    let term_logger = TermLogger::new(
        // TODO (Wybe 2022-07-16): Allow changing this through command line arguments
        LevelFilter::Info,
        config.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    create_dir_all(&log_dir)
        .unwrap_or_else(|_| panic!("Could not create all directories for `{}`", &log_dir));

    let date = chrono::offset::Local::today();
    let file_name = format!("{}/rss_r_{}.log", log_dir, date.format("%Y-%m-%d"));

    // We open the log file in append mode, so we don't overwrite any logs might already be there.
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_name)
        .unwrap_or_else(|_| panic!("Could not open `{}` for writing", file_name));

    let file_logger = WriteLogger::new(LevelFilter::Info, config, log_file);

    // We log both to the terminal, and to a file.
    CombinedLogger::init(vec![term_logger, file_logger]).unwrap();
}
