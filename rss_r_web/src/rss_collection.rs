use crate::requests::{ApiEndpoint, Requests, Response};
use egui::{Align2, Button, Context, TextEdit, Ui, Vec2};
use log::{info, warn};
use rss_com_lib::body::{AddFeedRequest, IsUrlAnRssFeedRequest, IsUrlAnRssFeedResponse};
use std::collections::HashMap;

/// Stores info about the rss feeds the user is following.
/// Is updated by information received from the server.
#[derive(Default)]
pub struct RssCollection {
    /// url -> Feed
    feeds: HashMap<String, RssFeed>,
    add_feed_popup: Option<AddFeedPopup>,
}

impl RssCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show_list(&mut self, ctx: &Context, ui: &mut Ui, requests: &mut Requests) {
        if ui.button("Add feed").clicked() && self.add_feed_popup.is_none() {
            self.add_feed_popup = Some(AddFeedPopup::new());
        }

        let close_popup = if let Some(popup) = &mut self.add_feed_popup {
            popup.show(ctx, requests)
        } else {
            false
        };
        if close_popup {
            self.add_feed_popup = None;
        }

        for (_, feed) in self.feeds.iter() {
            ui.label(&feed.name);
        }
    }
}

struct RssFeed {
    name: String,
}

#[derive(Default)]
struct AddFeedPopup {
    input_url: String,
    /// Result<(url, name), error_message>
    /// Name retrieved from the rss feed.
    /// The url is saved separately from the url input by the user, because they can change it
    /// at any point, and then it might not be a valid rss url anymore.
    /// TODO (Wybe 2022-07-14): Provision for multiple feeds being available?
    response: Option<Result<(String, String), String>>,
}

impl AddFeedPopup {
    fn new() -> Self {
        Default::default()
    }

    /// Returns whether it should be closed or not. False means to close the window.
    fn show(&mut self, ctx: &Context, requests: &mut Requests) -> bool {
        let mut is_open = true;
        let mut feed_was_added = false;

        egui::Window::new("Add feed")
            .open(&mut is_open)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                self.show_url_input(ui, requests);

                if let Some(response) = &self.response {
                    match response {
                        Ok((url, name)) => {
                            ui.label(format!("Feed found: {}", name));

                            AddFeedPopup::show_add_feed_button(ui, requests, url);
                        }
                        Err(error_message) => {
                            ui.colored_label(egui::Color32::RED, error_message);
                        }
                    }
                }

                if requests.has_request(ApiEndpoint::AddFeed) {
                    if let Some(response) = requests.ready(ApiEndpoint::AddFeed) {
                        match response {
                            Response::Ok(_) => {
                                // Success.
                                feed_was_added = true;
                            }
                            // TODO (Wybe 2022-07-16): Add error reporting.
                            Response::NotOk(_) => {}
                            Response::Error => {}
                        }
                    } else {
                        ui.spinner();
                    }
                }
            });

        if feed_was_added {
            is_open = false;
        }

        // TODO (Wybe 2022-07-16): Somehow signal that we added a new feed. So the list of feeds should be updated.
        !is_open
    }

    fn show_url_input(&mut self, ui: &mut Ui, requests: &mut Requests) {
        let test_request_ongoing = requests.has_request(ApiEndpoint::IsUrlAnRssFeed);

        ui.horizontal(|ui| {
            let url_edit_response = TextEdit::singleline(&mut self.input_url)
                .hint_text("Url")
                .show(ui)
                .response;

            let url_test_button_clicked = ui
                .add_enabled(!test_request_ongoing, Button::new("Test"))
                .clicked();

            if !test_request_ongoing
                && (url_test_button_clicked
                    || (url_edit_response.lost_focus() && ui.input().key_pressed(egui::Key::Enter)))
            {
                let request_body = IsUrlAnRssFeedRequest {
                    url: self.input_url.clone(),
                };
                requests.new_request_with_json_body(ApiEndpoint::IsUrlAnRssFeed, &request_body);

                self.response = None;
            }
        });

        if test_request_ongoing {
            if let Some(response) = requests.ready(ApiEndpoint::IsUrlAnRssFeed) {
                //TODO (Wybe 2022-07-16): Make some form of `ready` call that immediately checks for the response to be OK, and deserializes it to the requested type.
                if let Response::Ok(body) = response {
                    if let Ok(rss_response) = serde_json::from_str::<IsUrlAnRssFeedResponse>(&body)
                    {
                        match rss_response.result {
                            Ok(name) => {
                                self.response = Some(Ok((rss_response.requested_url, name)));
                            }
                            Err(err) => {
                                self.response = Some(Err(format!("No rss feed found: {}", err)));
                            }
                        }
                    }
                } else {
                    warn!(
                        "Something went wrong while testing the rss feed. Response was: {:?}",
                        response
                    );
                    self.response = Some(Err(
                        "Something went wrong while testing for an rss feed.".to_string(),
                    ));
                }
            } else {
                ui.spinner();
            }
        }
    }

    fn show_add_feed_button(ui: &mut Ui, requests: &mut Requests, feed_url: &str) {
        if ui
            .add_enabled(
                !requests.has_request(ApiEndpoint::AddFeed),
                Button::new("Add"),
            )
            .clicked()
        {
            requests.new_request_with_json_body(
                ApiEndpoint::AddFeed,
                AddFeedRequest {
                    url: feed_url.to_string(),
                },
            );
        }
    }
}
