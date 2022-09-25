#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod auth;
mod auth_middleware;
mod error;
mod persistence;
mod rss_cache;
mod rss_collection;
mod users;

use crate::auth::{AuthData, AUTH_COOKIE_NAME};
use crate::auth_middleware::{AuthenticateMiddlewareFactory, Authenticated};
use crate::persistence::SaveInRonFile;
use crate::rss_cache::RssCache;
use crate::rss_collection::RssCollections;
use crate::users::UserInfo;
use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use actix_web_lab::web::redirect;
use log::{error, info, warn, LevelFilter};
use rustls::{Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs::File;
use std::io::BufReader;

/// TODO (Wybe 2022-07-10): Add configuration options for ip address and port.
const IP: &str = "127.0.0.1:8443";
const LOGIN_DEADLINE: Duration = Duration::days(3);

/// TODO (Wybe 2022-07-10): Add some small banner that says this site uses cookies to authenticate? or is it not needed for authentication cookies.
/// TODO (Wybe 2022-07-12): Rss apparently sometimes allows getting push notifications, via a "Cloud" element in the feed. Is it worth it to implement this?
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO (Wybe 2022-07-10): Add application arguments or a config file that allows logging to
    //                         a file.
    TermLogger::init(
        // TODO (Wybe 2022-07-16): Allow changing this through command line arguments
        LevelFilter::Info,
        ConfigBuilder::default()
            .set_thread_level(LevelFilter::Trace)
            .set_target_level(LevelFilter::Trace)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let auth_data = AuthData::load_or_default();
    auth_data.save();

    // TODO (Wybe 2022-07-12): Is it a problem to store the auth data as web data?
    //                         all services would be able to access it. But the services
    //                         are programmed by us, so is there a way for an outsider to exploit that?
    //                         It does increase the probability of mistakes to slip in i think.
    let web_auth_data = web::Data::new(auth_data);

    // TODO (Wybe 2022-07-16): Check whether all users that have a collection actually exist.
    let rss_collections = RssCollections::load_or_default();
    rss_collections.save();
    let web_rss_collections = web::Data::new(rss_collections);

    let rustls_config = load_rustls_config();

    info!("Starting Https server at https://{IP}");

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

        App::new()
            .wrap(Logger::default())
            .service(redirect("/", "/app/index.html"))
            .service(redirect("/app/", "/app/index.html"))
            // This serves the static files of the rss_r_web webassembly application.
            .service(Files::new("/app", "resources/static"))
            .service(
                web::scope("/api")
                    .app_data(web_auth_data.clone())
                    .app_data(web_rss_collections.clone())
                    .app_data(web::Data::new(RssCache::default()))
                    .wrap(AuthenticateMiddlewareFactory)
                    .wrap(IdentityService::new(identity_policy))
                    .service(auth::test_auth_cookie)
                    .service(auth::login)
                    .service(auth::logout)
                    .service(rss_collection::is_url_an_rss_feed)
                    .service(rss_collection::get_feed_entries)
                    .service(rss_collection::add_feed)
                    .service(rss_collection::list_feeds),
            )
    })
    .bind_rustls(IP, rustls_config)?
    .run()
    .await
}

fn load_rustls_config() -> rustls::ServerConfig {
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    // TODO (Wybe 2022-07-10): Allow selecting certification files via a config file or maybe command line parameters.
    let cert_file = &mut BufReader::new(File::open("resources/local-ssl/localhost.pem").unwrap());
    let key_file =
        &mut BufReader::new(File::open("resources/local-ssl/localhost-key.pem").unwrap());

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        error!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()
}
