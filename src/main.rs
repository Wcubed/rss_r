#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

use actix_files::Files;
use actix_web::cookie::{Cookie, SameSite};
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_lab::web::redirect;
use log::{error, info, LevelFilter};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;

const AUTH_COOKIE_NAME: &str = "auth";

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

    info!("Starting Https server at https://127.0.0.1:8443");

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(counter.clone())
            .service(redirect("/", "/app/index.html"))
            .service(redirect("/app/", "/app/index.html"))
            // This serves the rss_r_web webassembly application.
            .service(Files::new("/app", "resources/static"))
            .service(
                web::scope("/api")
                    .service(login)
                    .service(logout)
                    .service(hello_world),
            )
    })
    .bind_rustls("127.0.0.1:8443", rustls_config)?
    .run()
    .await
}

#[get("/")]
async fn hello_world(data: web::Data<AppStateCounter>, req: HttpRequest) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;

    if let Some(cookie) = req.cookie(AUTH_COOKIE_NAME) {
        info!("Cookie: {}", cookie.value())
    }

    HttpResponse::Ok().body(format!("Hello world! Counter: {counter}"))
}

/// Validates user id and password, and if they are valid sets the authentication cookie.
#[get("/login")]
async fn login() -> impl Responder {
    // TODO (Wybe 2022-07-10): Add middleware for checking a login token.
    let auth_cookie = auth_cookie("test-token");

    HttpResponse::Ok().cookie(auth_cookie).finish()
}

/// Removes the authentication cookie, if it exists.
#[get("/logout")]
async fn logout(req: HttpRequest) -> impl Responder {
    if let Some(mut cookie) = req.cookie(AUTH_COOKIE_NAME) {
        // TODO (Wybe 2022-07-10): Invalidate this authentication token.
        cookie.make_removal();

        HttpResponse::Ok().cookie(cookie).finish()
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

fn auth_cookie(token: &str) -> Cookie {
    Cookie::build(AUTH_COOKIE_NAME, token)
        // Don't send this over unsecure channels.
        .secure(true)
        // Only send this if on the same site.
        .same_site(SameSite::Strict)
        // Javascript is not allowed to see this.
        .http_only(true)
        .finish()
}

struct AppStateCounter {
    counter: Mutex<i32>,
}

fn load_rustls_config() -> rustls::ServerConfig {
    let config = ServerConfig::builder()
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
