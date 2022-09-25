use crate::hyperlink::NewTabHyperlink;
use crate::requests::{ApiEndpoint, Requests, Response};
use chrono::Local;
use egui::{Align2, Button, Context, TextEdit, Ui, Vec2};
use log::warn;
use rss_com_lib::message_body::{
    AddFeedRequest, GetFeedEntriesRequest, GetFeedEntriesResponse, IsUrlAnRssFeedRequest,
    IsUrlAnRssFeedResponse, ListFeedsResponse,
};
use rss_com_lib::rss_feed::{EntryKey, FeedEntries, FeedEntry, FeedInfo};
use rss_com_lib::Url;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Stores info about the rss feeds the user is following.
/// Is updated by information received from the server.
#[derive(Default)]
pub struct RssCollection {
    /// url -> Feed
    /// If the entries are [None] that means we have not requested the feed entries from the server.
    /// TODO (Wybe 2022-07-18): Add a refresh button somewhere.
    feeds: HashMap<Url, (FeedInfo, Option<FeedEntries>)>,
    /// Url of selected feed.
    feed_selection: FeedSelection,
    /// A subset of all the entries in the `feeds` hashmap.
    selected_feed_entries: Vec<DisplayFeedEntry>,
    add_feed_popup: Option<AddFeedPopup>,
}

impl RssCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show_list(&mut self, ctx: &Context, ui: &mut Ui, requests: &mut Requests) {
        if let Some(popup) = &mut self.add_feed_popup {
            match popup.show(ctx, requests) {
                AddFeedPopupResponse::None => {} // Nothing to do.
                AddFeedPopupResponse::ClosePopup => {
                    self.add_feed_popup = None;
                }
                AddFeedPopupResponse::FeedAdded => {
                    self.add_feed_popup = None;
                    requests.new_request_without_body(ApiEndpoint::ListFeeds);
                }
            }
        }

        let previous_selection = self.feed_selection.clone();

        ui.selectable_value(&mut self.feed_selection, FeedSelection::All, "All feeds");

        ui.separator();

        for (url, (info, _)) in self.feeds.iter() {
            ui.selectable_value(
                &mut self.feed_selection,
                FeedSelection::Feed(url.clone()),
                &info.name,
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
                        for (url, info) in feeds_response.feeds.iter() {
                            if !self.feeds.contains_key(url) {
                                self.feeds.insert(url.clone(), (info.clone(), None));
                            }
                        }

                        // We received knowledge of what feeds are in the users collection.
                        // So we want to update the users currently viewed entries based
                        // on this new info.
                        self.update_feed_entries_based_on_selection(requests);

                        // TODO (Wybe 2022-07-16): Remove those no longer listed.
                        // TODO (Wybe 2022-07-16): update any existing feeds?
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
                            if let Ok((info, entries)) = result {
                                // The server always sends the complete state of the feed,
                                // so we don't have to worry about checking if the feed is already
                                // in the map. We can simply overwrite if if it exists.
                                self.feeds.insert(url, (info, Some(entries)));
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
                        for disp in entries
                            .iter()
                            .skip(row_range.start)
                            //TODO (Wybe 2022-07-18): Vertical scroll bar changes size sometimes during scrolling, why?
                            .take(row_range.end - row_range.start)
                        {
                            ui.label(&disp.entry.title);

                            ui.label(&disp.pub_date_string);

                            ui.label(&disp.feed_title);

                            if let Some(link) = &disp.entry.link {
                                ui.add(NewTabHyperlink::from_label_and_url("Open", link));
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
            if let Some((feed_info, maybe_entries)) = self.feeds.get(url) {
                if let Some(entries) = maybe_entries {
                    // TODO (Wybe 2022-07-19): there is probably a more efficient way than cloning everything.
                    for (key, entry) in entries.iter() {
                        self.selected_feed_entries.push(DisplayFeedEntry::new(
                            entry,
                            key,
                            feed_info.name.clone(),
                        ));
                    }
                } else {
                    // Feed's content not known, request from server.
                    urls_to_request.insert(url.clone());
                }
            }
        }

        self.selected_feed_entries.sort();

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

#[derive(Default)]
struct AddFeedPopup {
    input_url: String,
    /// Result<(url, name), error_message>
    /// Name retrieved from the rss feed.
    /// The url is saved separately from the url input by the user, because they can change it
    /// at any point, and then it might not be a valid rss url anymore.
    /// TODO (Wybe 2022-07-14): Provision for multiple feeds being available?
    response: Option<Result<(Url, String), String>>,
}

impl AddFeedPopup {
    fn new() -> Self {
        Default::default()
    }

    fn show(&mut self, ctx: &Context, requests: &mut Requests) -> AddFeedPopupResponse {
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

    fn show_add_feed_button(ui: &mut Ui, requests: &mut Requests, feed_url: &Url, feed_name: &str) {
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
                    name: feed_name.to_string(),
                },
            );
        }
    }
}

enum AddFeedPopupResponse {
    /// Nothing to do.
    None,
    /// User wants to close the popup. No new feeds.
    ClosePopup,
    /// User has added an rss feed. Update the list.
    FeedAdded,
}

#[derive(Clone, Eq, PartialEq)]
pub enum FeedSelection {
    /// Selects all feeds the user has.
    All,
    /// Selects one specific feed, based on it's url.
    Feed(Url),
}

impl Default for FeedSelection {
    fn default() -> Self {
        Self::All
    }
}

/// The info used to display an entry, so that it doesn't need to be recalculated each frame.
struct DisplayFeedEntry {
    entry: FeedEntry,
    /// Key to use when sending update requests to the server, such as marking the entry as read.
    key: EntryKey,
    /// Title of the feed this entry belongs to.
    feed_title: String,
    pub_date_string: String,
}

impl DisplayFeedEntry {
    fn new(entry: &FeedEntry, key: &EntryKey, feed_title: String) -> Self {
        DisplayFeedEntry {
            entry: entry.clone(),
            key: key.clone(),
            feed_title,
            pub_date_string: entry
                .pub_date
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string(),
        }
    }
}

impl PartialEq for DisplayFeedEntry {
    fn eq(&self, other: &Self) -> bool {
        self.entry.eq(&other.entry)
    }
}

impl Eq for DisplayFeedEntry {}

impl PartialOrd for DisplayFeedEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DisplayFeedEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entry.cmp(&other.entry)
    }
}
