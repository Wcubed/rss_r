use crate::requests::{ApiEndpoint, Requests, Response};
use chrono::Local;
use egui::{Align2, Button, Context, TextEdit, Ui, Vec2};
use log::warn;
use rss_com_lib::body::{
    AddFeedRequest, GetFeedEntriesRequest, GetFeedEntriesResponse, IsUrlAnRssFeedRequest,
    IsUrlAnRssFeedResponse, ListFeedsResponse,
};
use rss_com_lib::FeedEntry;
use std::collections::{HashMap, HashSet};

/// Stores info about the rss feeds the user is following.
/// Is updated by information received from the server.
#[derive(Default)]
pub struct RssCollection {
    /// url -> Feed
    /// TODO (Wybe 2022-07-18): Add a refresh button somewhere.
    feeds: HashMap<String, RssFeed>,
    /// Url of selected feed.
    feed_selection: FeedSelection,
    /// A subset of all the entries in the `feeds` hashmap.
    selected_feed_entries: Vec<FeedEntry>,
    add_feed_popup: Option<AddFeedPopup>,
}

impl RssCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show_list(&mut self, ctx: &Context, ui: &mut Ui, requests: &mut Requests) {
        let close_popup = if let Some(popup) = &mut self.add_feed_popup {
            popup.show(ctx, requests)
        } else {
            false
        };
        if close_popup {
            self.add_feed_popup = None;

            // After the popup is closed, we should check for any new feeds.
            // Because we might have just added a new feed.
            // TODO (Wybe 2022-07-16): Only do this if we indeed added a new feed.
            requests.new_empty_request(ApiEndpoint::ListFeeds);
        }

        let previous_selection = self.feed_selection.clone();

        ui.selectable_value(&mut self.feed_selection, FeedSelection::All, "All feeds");

        ui.separator();

        for (url, feed) in self.feeds.iter() {
            ui.selectable_value(
                &mut self.feed_selection,
                FeedSelection::Feed(url.clone()),
                &feed.name,
            );
        }

        if previous_selection != self.feed_selection {
            self.update_feed_entries_based_on_selection(requests);
        }

        ui.separator();
        if ui.button("Add feed").clicked() && self.add_feed_popup.is_none() {
            self.add_feed_popup = Some(AddFeedPopup::new());
        }

        if requests.has_request(ApiEndpoint::ListFeeds) {
            if let Some(response) = requests.ready(ApiEndpoint::ListFeeds) {
                // TODO (Wybe 2022-07-16): Handle errors
                if let Response::Ok(body) = response {
                    if let Ok(feeds_response) = serde_json::from_str::<ListFeedsResponse>(&body) {
                        // Add new feeds
                        for (url, name) in feeds_response.feeds.iter() {
                            if !self.feeds.contains_key(url) {
                                self.feeds.insert(
                                    url.clone(),
                                    RssFeed {
                                        name: name.clone(),
                                        entries: None,
                                    },
                                );
                            }
                        }

                        // We received knowledge of what feeds are in the users collection.
                        // So we want to update the users currently viewed entries based
                        // on this new info.
                        self.update_feed_entries_based_on_selection(requests);

                        // TODO (Wybe 2022-07-16): Remove those no longer listed.
                        // TODO (Wybe 2022-07-16): update any existing feeds.
                    }
                }
            } else {
                ui.spinner();
            }
        }
    }

    pub fn show_feed_entries(&mut self, ui: &mut Ui, requests: &mut Requests) {
        if requests.has_request(ApiEndpoint::GetFeedEntries) {
            if let Some(response) = requests.ready(ApiEndpoint::GetFeedEntries) {
                // TODO (Wybe 2022-07-16): Handle errors
                // TODO (Wybe 2022-07-18): Reduce nesting
                if let Response::Ok(body) = response {
                    if let Ok(feeds_response) =
                        serde_json::from_str::<GetFeedEntriesResponse>(&body)
                    {
                        for (url, result) in feeds_response.results {
                            if let Ok(entries) = result {
                                if let Some(feed) = self.feeds.get_mut(&url) {
                                    feed.entries = Some(entries);
                                }
                            }
                        }

                        self.update_feed_entries_based_on_selection(requests);
                    }
                }
            } else {
                ui.spinner();
            }
        }

        let entries = &self.selected_feed_entries;

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, entries.len(), |ui, row_range| {
                egui::Grid::new("feed-grid")
                    .striped(true)
                    .num_columns(3)
                    .start_row(row_range.start)
                    .show(ui, |ui| {
                        for entry in entries
                            .iter()
                            .skip(row_range.start)
                            //TODO (Wybe 2022-07-18): Vertical scroll bar changes size sometimes during scrolling, why?
                            .take(row_range.end - row_range.start)
                        {
                            ui.label(&entry.title);

                            if let Some(pub_date) = &entry.pub_date {
                                // TODO Wybe: How to make this display local time `.with_timezone(&Local)` seems to still give +0 offset, instead of the +2 it should give.
                                // TODO: Show "x hours ago" or "x days ago" instead of the date and time, when the entry is recent.
                                ui.label(
                                    &pub_date
                                        .with_timezone(&Local)
                                        .format("%Y-%m-%d")
                                        .to_string(),
                                );
                            } else {
                                ui.label("");
                            }

                            if let Some(link) = &entry.link {
                                ui.add(egui::Hyperlink::from_label_and_url("Open", link));
                            } else {
                                // No link, so add an empty label to skip this column.
                                ui.label("");
                            }

                            ui.end_row();
                        }
                    });
            });
    }

    fn update_feed_entries_based_on_selection(&mut self, requests: &mut Requests) {
        self.selected_feed_entries = Vec::new();

        // TODO (Wybe 2022-07-18): Queue request if another request is outgoing.
        // TODO (Wybe 2022-07-18): Change the main display already to whatever we have loaded from our local storage?
        //                          and update the display when the request returns.
        let mut urls_to_display = HashSet::new();
        match &self.feed_selection {
            FeedSelection::All => {
                for url in self.feeds.keys() {
                    urls_to_display.insert(url);
                }
            }
            FeedSelection::Feed(url) => {
                urls_to_display.insert(url);
            }
        }

        let mut urls_to_request = HashSet::new();

        // Check which feeds we already have the items of,
        // and therefore don't need to request from the server.
        for &url in urls_to_display.iter() {
            if let Some(feed) = self.feeds.get(url) {
                if let Some(entries) = &feed.entries {
                    // TODO (Wybe 2022-07-19): there is probably a more efficient way than cloning everything.
                    self.selected_feed_entries.extend(entries.iter().cloned());
                } else {
                    // Feed's content not known, request from server.
                    urls_to_request.insert(url.clone());
                }
            }
        }

        if !urls_to_request.is_empty() {
            requests.new_request_with_json_body(
                ApiEndpoint::GetFeedEntries,
                GetFeedEntriesRequest {
                    feeds: urls_to_request,
                },
            )
        }
    }
}

struct RssFeed {
    name: String,
    entries: Option<Vec<FeedEntry>>,
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

                            AddFeedPopup::show_add_feed_button(ui, requests, url, name);
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

    fn show_add_feed_button(ui: &mut Ui, requests: &mut Requests, feed_url: &str, feed_name: &str) {
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
                    name: feed_name.to_string(),
                },
            );
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum FeedSelection {
    /// Selects all feeds the user has.
    All,
    /// Selects one specific feed, based on it's url.
    Feed(String),
}

impl Default for FeedSelection {
    fn default() -> Self {
        Self::All
    }
}
