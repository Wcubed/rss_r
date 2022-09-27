use crate::edit_feed_popup::TagSelector;
use crate::requests::{ApiEndpoint, Requests, Response};
use crate::{POPUP_ALIGN, POPUP_OFFSET};
use egui::{Align2, Button, Context, TextEdit, Ui, Vec2};
use log::warn;
use rss_com_lib::message_body::{AddFeedRequest, IsUrlAnRssFeedRequest, IsUrlAnRssFeedResponse};
use rss_com_lib::rss_feed::FeedInfo;
use rss_com_lib::Url;
use std::collections::HashSet;

pub struct AddFeedPopup {
    input_url: String,
    /// Result<(url, name), error_message>
    /// Name retrieved from the rss feed.
    /// The url is saved separately from the url input by the user, because they can change it
    /// at any point, and then it might not be a valid rss url anymore.
    /// TODO (Wybe 2022-07-14): Provision for multiple feeds being available?
    feed_test_response: Option<Result<(Url, String), String>>,
    tag_selector: TagSelector,
}

impl AddFeedPopup {
    pub fn new() -> Self {
        AddFeedPopup {
            input_url: "".to_string(),
            feed_test_response: None,
            // TODO (Wybe 2022-09-27): Add tags we already know about.
            tag_selector: TagSelector::new(&HashSet::new()),
        }
    }

    pub fn show(&mut self, ctx: &Context, requests: &mut Requests) -> AddFeedPopupResponse {
        let mut is_open = true;
        let mut feed_was_added = false;

        egui::Window::new("Add feed")
            .open(&mut is_open)
            .anchor(POPUP_ALIGN, POPUP_OFFSET)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                self.show_url_input(ui, requests);

                if let Some(response) = &self.feed_test_response {
                    match response {
                        Ok((url, name)) => {
                            ui.label(format!("Feed found: {}", name));

                            self.tag_selector.show(ui);

                            AddFeedPopup::show_add_feed_button(
                                ui,
                                requests,
                                url,
                                name,
                                &self.tag_selector,
                            );
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
            AddFeedPopupResponse::FeedAdded
        } else if !is_open {
            AddFeedPopupResponse::ClosePopup
        } else {
            AddFeedPopupResponse::None
        }
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
                    url: Url::new(self.input_url.clone()),
                };
                requests.new_request_with_json_body(ApiEndpoint::IsUrlAnRssFeed, &request_body);

                self.feed_test_response = None;
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
                                self.feed_test_response =
                                    Some(Ok((rss_response.requested_url, name)));
                            }
                            Err(err) => {
                                self.feed_test_response =
                                    Some(Err(format!("No rss feed found: {}", err)));
                            }
                        }
                    }
                } else {
                    warn!(
                        "Something went wrong while testing the rss feed. Response was: {:?}",
                        response
                    );
                    self.feed_test_response = Some(Err(
                        "Something went wrong while testing for an rss feed.".to_string(),
                    ));
                }
            } else {
                ui.spinner();
            }
        }
    }

    fn show_add_feed_button(
        ui: &mut Ui,
        requests: &mut Requests,
        feed_url: &Url,
        feed_name: &str,
        tag_selector: &TagSelector,
    ) {
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
                    url: feed_url.clone(),
                    info: FeedInfo {
                        name: feed_name.to_string(),
                        tags: tag_selector.get_selected_tags(),
                    },
                },
            );
        }
    }
}

pub enum AddFeedPopupResponse {
    /// Nothing to do.
    None,
    /// User wants to close the popup. No new feeds.
    ClosePopup,
    /// User has added an rss feed. Update the list.
    FeedAdded,
}
