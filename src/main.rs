#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod auth;
mod auth_middleware;
mod error;
mod users;

use crate::auth::{AuthData, AUTH_COOKIE_NAME};
use crate::auth_middleware::{AuthenticateMiddlewareFactory, Authenticated};
use crate::users::{UserId, UserInfo};
use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web_lab::web::redirect;
use log::{error, info, warn, LevelFilter};
use rustls::{Certificate, PrivateKey};
use rustls_pemfile::{certs, pkcs8_private_keys};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;

/// TODO (Wybe 2022-07-10): Add configuration options for ip address and port.
const IP: &str = "127.0.0.1:8443";
const LOGIN_DEADLINE: Duration = Duration::days(3);

/// TODO (Wybe 2022-07-10): Can we save the token on the client in a (httpOnly, Secure, SameSite=Strict) cookie?
/// TODO (Wybe 2022-07-10): Allow authenticating non-hardcoded users.
/// TODO (Wybe 2022-07-10): Store the bearer token on the client site in http only cookies (aparently there are cookies that can only be sent along with requests, and not accessed).
/// TODO (Wybe 2022-07-10): Add some small banner that says this site uses cookies to authenticate? or is it not needed for authentication cookies.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TODO (Wybe 2022-07-10): Add application arguments or a config file that allows logging to
    //                         a file.
    TermLogger::init(
        LevelFilter::Info,
        ConfigBuilder::default()
            .set_thread_level(LevelFilter::Trace)
            .set_target_level(LevelFilter::Trace)
            .build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .unwrap();

    let rustls_config = load_rustls_config();

    let counter = web::Data::new(AppStateCounter {
        counter: Mutex::new(0),
    });

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

        let mut auth_data = AuthData::new();
        // TODO (Wybe 2022-07-12): Have some way of creating / storing users.
        auth_data.new_user(UserInfo {
            id: UserId(1),
            name: "test".to_string(),
            password: "testing".to_string(),
        });

        // TODO (Wybe 2022-07-12): Is it a problem to store the auth data as web data?
        //                         all services would be able to access it. But the services
        //                         are programmed by us, so is there a way for an outsider to exploit that?
        //                         It does increase the probability of mistakes to slip in i think.
        let web_auth_data = web::Data::new(auth_data);

        App::new()
            .wrap(Logger::default())
            .app_data(counter.clone())
            .service(redirect("/", "/app/index.html"))
            .service(redirect("/app/", "/app/index.html"))
            // This serves the static files of the rss_r_web webassembly application.
            .service(Files::new("/app", "resources/static"))
            .service(
                web::scope("/api")
                    .app_data(web_auth_data)
                    .wrap(AuthenticateMiddlewareFactory)
                    .wrap(IdentityService::new(identity_policy))
                    .service(auth::test_auth_cookie)
                    .service(auth::login)
                    .service(auth::logout)
                    .service(hello_world),
            )
    })
    .bind_rustls(IP, rustls_config)?
    .run()
    .await
}

#[get("/")]
async fn hello_world(data: web::Data<AppStateCounter>, _auth: Authenticated) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;

    HttpResponse::Ok().body(format!("Hello world! Counter: {counter}"))
}

struct AppStateCounter {
    counter: Mutex<i32>,
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
