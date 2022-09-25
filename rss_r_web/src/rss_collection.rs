use crate::add_feed_popup::{AddFeedPopup, AddFeedPopupResponse};
use crate::hyperlink::NewTabHyperlink;
use crate::requests::{ApiEndpoint, Requests, Response};
use chrono::Local;
use egui::{Color32, Context, Label, RichText, Ui};
use log::info;
use rss_com_lib::message_body::{
    GetFeedEntriesRequest, GetFeedEntriesResponse, ListFeedsResponse,
    SetEntryReadRequestAndResponse,
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

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.feed_selection, FeedSelection::All, "All feeds");
            if ui.button("Add feed").clicked() && self.add_feed_popup.is_none() {
                self.add_feed_popup = Some(AddFeedPopup::new());
            }
        });

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

        if requests.has_request(ApiEndpoint::SetEntryRead) {
            if let Some(Response::Ok(body)) = requests.ready(ApiEndpoint::SetEntryRead) {
                // `read` field was set successfully. Update the visuals to match.
                if let Ok(response) = serde_json::from_str::<SetEntryReadRequestAndResponse>(&body)
                {
                    if let Some((_, Some(feed))) = self.feeds.get_mut(&response.feed_url) {
                        if let Some(entry) = feed.get_mut(&response.entry_key) {
                            entry.read = response.read
                        }
                    }
                }

                // TODO (Wybe 2022-09-25): Can we do better than updating the complete list of entries?
                self.update_feed_entries_based_on_selection(requests);
            }
        }

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        let unread_entry_text_color = ui.ctx().style().visuals.strong_text_color();

        let mut set_entry_read_request = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show_rows(
                ui,
                row_height,
                self.selected_feed_entries.len(),
                |ui, row_range| {
                    egui::Grid::new("feed-grid")
                        .striped(true)
                        .num_columns(5)
                        .start_row(row_range.start)
                        .show(ui, |ui| {
                            for disp in self
                                .selected_feed_entries
                                .iter()
                                .skip(row_range.start)
                                //TODO (Wybe 2022-07-18): Vertical scroll bar changes size sometimes during scrolling, why?
                                .take(row_range.end - row_range.start)
                            {
                                let unread = !disp.entry.read;

                                let mut mark_read = !unread;
                                ui.checkbox(
                                    &mut mark_read,
                                    highlighted_text(
                                        &disp.entry.title,
                                        unread,
                                        unread_entry_text_color,
                                    ),
                                );

                                if mark_read == unread {
                                    // User wants to mark this entry as read or unread.
                                    set_entry_read_request = Some(SetEntryReadRequestAndResponse {
                                        feed_url: disp.feed_url.clone(),
                                        entry_key: disp.key.clone(),
                                        read: mark_read,
                                    });
                                }

                                ui.label(highlighted_text(
                                    &disp.pub_date_string,
                                    unread,
                                    unread_entry_text_color,
                                ));

                                ui.label(highlighted_text(
                                    &disp.feed_title,
                                    unread,
                                    unread_entry_text_color,
                                ));

                                if let Some(link) = &disp.entry.link {
                                    ui.add(NewTabHyperlink::from_label_and_url("Open", link));

                                    if ui
                                        .add(NewTabHyperlink::from_label_and_url(
                                            "Open mark read",
                                            link,
                                        ))
                                        .clicked()
                                        && !disp.entry.read
                                    {
                                        // User wants to mark this entry as read.
                                        set_entry_read_request =
                                            Some(SetEntryReadRequestAndResponse {
                                                feed_url: disp.feed_url.clone(),
                                                entry_key: disp.key.clone(),
                                                read: true,
                                            });
                                    }
                                } else {
                                    // No link, so add empty labels to skip these columns.
                                    ui.label("");
                                    ui.label("");
                                }

                                ui.end_row();
                            }
                        });
                },
            );

        if let Some(request) = set_entry_read_request {
            requests.new_request_with_json_body(ApiEndpoint::SetEntryRead, request);
        }
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
                            url.clone(),
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

fn highlighted_text(text: &str, highlight: bool, highlight_color: Color32) -> RichText {
    let mut text = RichText::new(text);
    if highlight {
        text = text.color(highlight_color);
    }
    text
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
    feed_url: Url,
    pub_date_string: String,
    read: bool,
}

impl DisplayFeedEntry {
    fn new(entry: &FeedEntry, key: &EntryKey, feed_title: String, feed_url: Url) -> Self {
        DisplayFeedEntry {
            entry: entry.clone(),
            key: key.clone(),
            feed_title,
            feed_url,
            pub_date_string: entry
                .pub_date
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string(),
            read: entry.read,
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
