#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod auth_middleware;
mod error;
mod users;

use crate::auth_middleware::{
    AuthData, AuthenticateMiddlewareFactory, Authenticated, AUTH_COOKIE_NAME,
};
use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, Identity, IdentityService};
use actix_web::cookie::time::Duration;
use actix_web::cookie::SameSite;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_lab::web::redirect;
use log::{error, info, warn, LevelFilter};
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};
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

        let auth_data = AuthData;

        App::new()
            .wrap(Logger::default())
            .app_data(counter.clone())
            .service(redirect("/", "/app/index.html"))
            .service(redirect("/app/", "/app/index.html"))
            // This serves the static files of the rss_r_web webassembly application.
            .service(Files::new("/app", "resources/static"))
            .service(
                web::scope("/api")
                    .wrap(AuthenticateMiddlewareFactory::new(auth_data))
                    .wrap(IdentityService::new(identity_policy))
                    .service(login)
                    .service(logout)
                    .service(hello_world),
            )
    })
    .bind_rustls(IP, rustls_config)?
    .run()
    .await
}

#[get("/")]
async fn hello_world(data: web::Data<AppStateCounter>, auth: Authenticated) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;

    HttpResponse::Ok().body(format!("Hello world! Counter: {counter}"))
}

/// Validates user id and password, and if they are valid sets the authentication cookie.
#[get("/login")]
async fn login(req: HttpRequest, id: Identity) -> impl Responder {
    // TODO (Wybe 2022-07-10): Add middleware for checking a login token.

    if let (Some(user_name), Some(password)) = (
        req.headers()
            .get(USER_ID_HEADER)
            .and_then(|id| id.to_str().ok()),
        req.headers().get(PASSWORD_HEADER),
    ) {
        // TODO (Wybe 2022-07-10): Allow registering and remembering users and such.
        if user_name == "test" && password == "testing" {
            info!("Logging in `{}`", user_name);
            // Login valid, set the auth cookie so the user doesn't need to login all the time.
            // TODO (Wybe 2022-07-11): Generate and remember the session id somewhere.
            id.remember("0".to_string());
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::Unauthorized().finish()
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

/// Removes the authentication cookie
#[get("/logout")]
async fn logout(id: Identity, auth: Authenticated) -> impl Responder {
    info!("Logging out `{}`", auth.user_name());

    id.forget();
    HttpResponse::Ok().finish()
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
