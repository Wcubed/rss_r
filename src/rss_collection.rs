use crate::Authenticated;
use actix_web::{post, web, HttpResponse, Responder};
use log::info;
use rss::Channel;
use rss_com_lib::body::{DoesFeedExistRequest, DoesFeedExistResponse};
use std::error::Error;

/// Checks a given rss feed for existence.
/// Sends back the title of the feed if it exists.
/// TODO (Wybe 2022-07-14): Sanitize url?
/// TODO (Wybe 2022-07-14): Can we do Rust object notation, instead of parsing from Json?
#[post("/does_feed_exist")]
pub async fn does_feed_exist(
    request: web::Json<DoesFeedExistRequest>,
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

    HttpResponse::Ok().json(DoesFeedExistResponse {
        requested_url: request.url.to_string(),
        result,
    })
}

async fn download_feed(url: &str) -> Result<Channel, Box<dyn Error>> {
    let content = reqwest::get(url).await?.bytes().await?;
    let channel = Channel::read_from(&content[..])?;
    Ok(channel)
}
