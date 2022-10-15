use crate::add_feed_popup::{AddFeedPopup, AddFeedPopupResponse};
use crate::edit_feed_popup::{EditFeedPopup, EditFeedPopupResponse};
use crate::hyperlink::NewTabHyperlink;
use crate::requests::{ApiEndpoint, Requests, Response};
use chrono::Local;
use egui::{Color32, RichText, Ui};
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
    feeds_display: FeedListDisplay,
    /// A subset of all the entries in the `feeds` hashmap.
    selected_feed_entries: Vec<DisplayFeedEntry>,
    /// Whether or not to show feed entries that have already been read.
    show_unread_entries: bool,
}

impl RssCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn show_feed_list(&mut self, ui: &mut Ui, requests: &mut Requests) {
        let last_show_read_entries = self.show_unread_entries;
        ui.checkbox(&mut self.show_unread_entries, "Show read entries");

        if last_show_read_entries != self.show_unread_entries {
            self.update_feed_entries_based_on_selection(requests);
        }

        if ui.button("Update selected feeds").clicked() {
            // Request the server to update all the feeds we have selected now,
            // and return the new entries.
            requests.new_request_with_json_body(
                ApiEndpoint::GetFeedEntries,
                GetFeedEntriesRequest {
                    feeds: self.get_urls_from_current_selection(),
                    refresh: true,
                },
            )
        }

        match self.feeds_display.show(ui, requests) {
            FeedListDisplayResponse::None => {} // Nothing to do
            FeedListDisplayResponse::FeedInfoEdited(url, new_info) => {
                if let Some(feed) = self.feeds.get_mut(&url) {
                    feed.0 = new_info;
                }

                self.feeds_display.update_feeds(&self.feeds);
            }
            FeedListDisplayResponse::SelectionChanged => {
                self.update_feed_entries_based_on_selection(requests)
            }
        }

        if requests.has_request(ApiEndpoint::ListFeeds) {
            if let Some(response) = requests.ready(ApiEndpoint::ListFeeds) {
                // TODO (Wybe 2022-07-16): Handle errors
                if let Response::Ok(body) = response {
                    if let Ok(feeds_response) = serde_json::from_str::<ListFeedsResponse>(&body) {
                        // Add new feeds and update existing ones.
                        for (url, info) in feeds_response.feeds.iter() {
                            if let Some(feed) = self.feeds.get_mut(url) {
                                // Feed exists. Update it's info.
                                feed.0 = info.clone();
                            } else {
                                self.feeds.insert(url.clone(), (info.clone(), None));
                            }
                        }

                        // TODO (Wybe 2022-09-25): Remove feeds that are no longer listed.

                        self.feeds_display.update_feeds(&self.feeds);

                        // We received knowledge of what feeds are in the users collection.
                        // So we want to update the users currently viewed entries based
                        // on this new info.
                        self.update_feed_entries_based_on_selection(requests);
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
                        for (url, (info, entries)) in feeds_response.results {
                            // The server always sends the complete state of the feed,
                            // so we don't have to worry about checking if the feed is already
                            // in the map. We can simply overwrite if if it exists.
                            self.feeds.insert(url, (info, Some(entries)));
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
                                        &disp.display_title,
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

                                    if !disp.read {
                                        // Item not read, so we add an option to open and mark it "read" at the same time.
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
                                        ui.label("");
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

    /// Checks what the user has selected, and returns all the urls that match the selection.
    fn get_urls_from_current_selection(&self) -> HashSet<Url> {
        let mut selected = HashSet::new();

        match self.feeds_display.get_current_selection() {
            FeedSelection::All => {
                for url in self.feeds.keys() {
                    selected.insert(url.clone());
                }
            }
            FeedSelection::Feed(url) => {
                selected.insert(url);
            }
            FeedSelection::Tag(_, urls) => {
                for url in urls {
                    selected.insert(url);
                }
            }
        }

        selected
    }

    fn update_feed_entries_based_on_selection(&mut self, requests: &mut Requests) {
        self.selected_feed_entries = Vec::new();

        // TODO (Wybe 2022-07-18): Queue request if another request is outgoing.
        // TODO (Wybe 2022-07-18): Change the main display already to whatever we have loaded from our local storage?
        //                          and update the display when the request returns.
        let mut urls_to_display = self.get_urls_from_current_selection();
        let mut urls_to_request = HashSet::new();

        // Check which feeds we already have the items of,
        // and therefore don't need to request from the server.
        for url in urls_to_display.iter() {
            if let Some((feed_info, maybe_entries)) = self.feeds.get(&url) {
                if let Some(entries) = maybe_entries {
                    // TODO (Wybe 2022-07-19): there is probably a more efficient way than cloning everything.
                    for (key, entry) in entries.iter() {
                        if entry.read && !self.show_unread_entries {
                            // This entry doesn't need to be shown, because it is already read.
                            continue;
                        }

                        self.selected_feed_entries.push(DisplayFeedEntry::new(
                            entry,
                            key,
                            &feed_info.name,
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
                    refresh: false,
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

/// The info used to display an entry, so that it doesn't need to be recalculated each frame.
struct DisplayFeedEntry {
    entry: FeedEntry,
    /// The feed title, but formatted for display.
    display_title: String,
    /// Key to use when sending update requests to the server, such as marking the entry as read.
    key: EntryKey,
    /// Title of the feed this entry belongs to.
    feed_title: String,
    feed_url: Url,
    pub_date_string: String,
    read: bool,
}

impl DisplayFeedEntry {
    fn new(entry: &FeedEntry, key: &EntryKey, feed_title: &str, feed_url: Url) -> Self {
        let display_title = cut_middle_of_string_if_too_long(&entry.title, 60);
        let feed_title = cut_middle_of_string_if_too_long(feed_title, 40);

        DisplayFeedEntry {
            entry: entry.clone(),
            display_title,
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

/// Cuts out the middle of strings if they are too long.
pub fn cut_middle_of_string_if_too_long(input: &str, max_length: usize) -> String {
    let infix = "....";
    let infix_length = 4;

    let string_length = input.chars().count();

    if string_length > max_length {
        // String is too long. Snip it, and add the infix.
        // We show the first few, and last few characters, because the last few usually include
        // a chapter number. Which is important to show to the user.

        let mut result = String::with_capacity(max_length);
        let mut start_length = (max_length / 2) - (infix_length / 2);
        let end_length = start_length;

        if max_length % 2 == 1 {
            // Odd length. The odd char goes to the first section.
            start_length += 1;
        }

        for (i, c) in input.char_indices() {
            if i < start_length {
                result.push(c);
            }
            if i == start_length {
                result.push_str(infix);
            }
            if i >= string_length - end_length {
                result.push(c);
            }
        }

        result
    } else {
        input.to_string()
    }
}

#[derive(Default)]
struct FeedListDisplay {
    /// A copy of all the known feeds, but in a layout suited for quick display.
    /// Sorted list of tags -> Feeds.
    feed_tags: Vec<(String, Vec<(Url, FeedInfo)>)>,
    feeds_without_tags: Vec<(Url, FeedInfo)>,
    /// A copy of all known tags. For quick access.
    known_tags: HashSet<String>,
    selection: FeedSelection,
    add_feed_popup: Option<AddFeedPopup>,
    edit_feed_popup: Option<EditFeedPopup>,
}

impl FeedListDisplay {
    fn new() -> Self {
        Default::default()
    }

    fn update_feeds(&mut self, feeds: &HashMap<Url, (FeedInfo, Option<FeedEntries>)>) {
        let mut feeds_by_tag: HashMap<String, Vec<(Url, FeedInfo)>> = HashMap::new();
        self.feeds_without_tags = Vec::new();
        self.known_tags = HashSet::new();

        // TODO (Wybe 2022-09-27): Check if the currently selected feed still exists, and act accordingly if it doesn't.

        // Collect all the feeds per tag.
        for (url, (info, _)) in feeds.iter() {
            for tag in info.tags.iter() {
                if let Some(feeds_with_tag) = feeds_by_tag.get_mut(tag) {
                    feeds_with_tag.push((url.clone(), info.clone()));
                } else {
                    feeds_by_tag.insert(tag.clone(), vec![(url.clone(), info.clone())]);
                    self.known_tags.insert(tag.clone());
                }
            }

            if info.tags.is_empty() {
                // This feed has no tags.
                self.feeds_without_tags.push((url.clone(), info.clone()));
            }
        }

        // Sort the feeds per tag.
        for feeds in feeds_by_tag.values_mut() {
            feeds.sort_by(|(_, this_info), (_, other_info)| this_info.name.cmp(&other_info.name));
        }
        self.feeds_without_tags
            .sort_by(|(_, this_info), (_, other_info)| this_info.name.cmp(&other_info.name));

        // Update selection
        self.selection = match &self.selection {
            FeedSelection::Tag(tag, _) => {
                if let Some(feeds) = feeds_by_tag.get(tag) {
                    FeedSelection::Tag(
                        tag.clone(),
                        feeds.iter().map(|(url, _info)| url.clone()).collect(),
                    )
                } else {
                    // Selected tag no longer exists.
                    FeedSelection::All
                }
            }
            selection => selection.clone(),
        };

        // Sort the tags.
        let mut sorted_by_tag: Vec<(String, Vec<(Url, FeedInfo)>)> =
            feeds_by_tag.into_iter().collect();
        sorted_by_tag.sort_by(|(tag, _), (other_tag, _)| tag.cmp(other_tag));

        self.feed_tags = sorted_by_tag;
    }

    fn get_current_selection(&self) -> FeedSelection {
        self.selection.clone()
    }

    fn show(&mut self, ui: &mut Ui, requests: &mut Requests) -> FeedListDisplayResponse {
        let mut response = FeedListDisplayResponse::None;

        if ui.button("Add feed").clicked() && self.add_feed_popup.is_none() {
            self.add_feed_popup = Some(AddFeedPopup::new(self.known_tags.clone()));
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if selectable_value(ui, self.selection == FeedSelection::All, "All feeds") {
                self.selection = FeedSelection::All;
                response = FeedListDisplayResponse::SelectionChanged;
            }

            // TODO (Wybe 2022-09-27): Deduplicate code.
            if !self.feeds_without_tags.is_empty() {
                ui.collapsing("Untagged", |ui| {
                    for (url, info) in self.feeds_without_tags.iter() {
                        let selected = match &self.selection {
                            FeedSelection::Feed(selected_url) => selected_url == url,
                            _ => false,
                        };

                        ui.horizontal(|ui| {
                            ui.label("-");
                            ui.horizontal_wrapped(|ui| {
                                if selectable_value(ui, selected, &info.name) {
                                    self.selection = FeedSelection::Feed(url.clone());
                                    response = FeedListDisplayResponse::SelectionChanged;
                                }

                                // Only show the edit buton if the feed is selected.
                                if selected
                                    && ui.button("Edit").clicked()
                                    && self.edit_feed_popup.is_none()
                                {
                                    self.edit_feed_popup = Some(EditFeedPopup::new(
                                        url.clone(),
                                        info.clone(),
                                        self.known_tags.clone(),
                                    ));
                                }
                            });
                        });
                    }
                });
            }

            for (tag, feeds) in self.feed_tags.iter() {
                ui.collapsing(tag, |ui| {
                    let tag_selected = match &self.selection {
                        FeedSelection::Tag(selected_tag, _) => selected_tag == tag,
                        _ => false,
                    };

                    if selectable_value(ui, tag_selected, "All") {
                        self.selection = FeedSelection::Tag(
                            tag.clone(),
                            feeds.iter().map(|(url, _)| url.clone()).collect(),
                        );

                        response = FeedListDisplayResponse::SelectionChanged;
                    }

                    for (url, info) in feeds {
                        let selected = match &self.selection {
                            FeedSelection::Feed(selected_url) => selected_url == url,
                            _ => false,
                        };

                        ui.horizontal(|ui| {
                            ui.label("-");
                            ui.horizontal_wrapped(|ui| {
                                if selectable_value(ui, selected, &info.name) {
                                    self.selection = FeedSelection::Feed(url.clone());
                                    response = FeedListDisplayResponse::SelectionChanged;
                                }

                                // Only show the edit buton if the feed is selected.
                                if selected
                                    && ui.button("Edit").clicked()
                                    && self.edit_feed_popup.is_none()
                                {
                                    self.edit_feed_popup = Some(EditFeedPopup::new(
                                        url.clone(),
                                        info.clone(),
                                        self.known_tags.clone(),
                                    ));
                                }
                            });
                        });
                    }
                });
            }
        });

        // Handle "Add feed" popup
        if let Some(popup) = &mut self.add_feed_popup {
            match popup.show(ui.ctx(), requests) {
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

        // Handle "edit feed info" popup.
        if let Some(popup) = &mut self.edit_feed_popup {
            match popup.show(ui.ctx(), requests) {
                EditFeedPopupResponse::None => {} // Nothing to do.
                EditFeedPopupResponse::ClosePopup => {
                    self.edit_feed_popup = None;
                }
                EditFeedPopupResponse::FeedInfoEdited(url, new_info) => {
                    // Edit was a success. Close the popup.
                    self.edit_feed_popup = None;

                    response = FeedListDisplayResponse::FeedInfoEdited(url, new_info);
                }
            }
        }

        response
    }
}

pub enum FeedListDisplayResponse {
    None,
    FeedInfoEdited(Url, FeedInfo),
    SelectionChanged,
}

#[derive(Clone, Eq, PartialEq)]
pub enum FeedSelection {
    /// Selects all feeds the user has.
    All,
    Tag(String, Vec<Url>),
    /// Selects one specific feed, based on it's url.
    Feed(Url),
}

impl Default for FeedSelection {
    fn default() -> Self {
        Self::All
    }
}

/// A selectable value that will return true if it has been selected by the user.
fn selectable_value(ui: &mut Ui, mut selected: bool, text: &str) -> bool {
    ui.toggle_value(&mut selected, text).clicked()
}

#[cfg(test)]
mod tests {
    use crate::rss_collection::cut_middle_of_string_if_too_long;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("This is a very long string", 12, "This....ring")]
    #[case("Uncut string", 40, "Uncut string")]
    #[case("1234567890", 8, "12....90")]
    #[case("An odd string length", 7, "An....h")]
    fn test_cut_middle_of_string_if_too_long(
        #[case] input: &str,
        #[case] max_length: usize,
        #[case] expected: &str,
    ) {
        let result = cut_middle_of_string_if_too_long(input, max_length);
        assert_eq!(&result, expected);
    }
}
