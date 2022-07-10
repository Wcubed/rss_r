#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use std::sync::Mutex;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let counter = web::Data::new(AppStateCounter {
        counter: Mutex::new(0),
    });

    HttpServer::new(move || App::new().app_data(counter.clone()).service(hello_world))
        .bind(("127.0.0.1", 8080))?
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
