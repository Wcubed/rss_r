#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

use actix_files::Files;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use log::{error, info, LevelFilter};
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use simplelog::{ColorChoice, ConfigBuilder, TermLogger, TerminalMode};
use std::fs::File;
use std::io::BufReader;
use std::sync::Mutex;

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
            .service(hello_world)
            .service(Files::new("/static", "static"))
    })
    .bind_rustls("127.0.0.1:8443", rustls_config)?
    .run()
    .await
}

#[get("/")]
async fn hello_world(data: web::Data<AppStateCounter>) -> impl Responder {
    let mut counter = data.counter.lock().unwrap();
    *counter += 1;

    HttpResponse::Ok().body(format!("Hello world! Counter: {counter}"))
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
