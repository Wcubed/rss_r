use crate::feed_list_display::{FeedListDisplay, FeedListDisplayResponse, FeedListPopupResponse};
use crate::hyperlink::NewTabHyperlink;
use crate::requests::{ApiEndpoint, Requests, Response};
use chrono::Local;
use egui::{Color32, RichText, Ui, Vec2};
use rss_com_lib::message_body::{
    AdditionalAction, ComFeedEntry, EntryTypeFilter, FeedsRequest, FeedsResponse,
    SetEntryReadRequestAndResponse,
};
use rss_com_lib::rss_feed::{EntryKey, FeedInfo};
use rss_com_lib::Url;
use std::collections::HashMap;
use std::fmt::format;

const SIDEPANEL_COLLAPSE_WIDTH: f32 = 900.0;
const DEFAULT_ENTRY_REQUEST_AMOUNT: usize = 30;

/// Stores info about the rss feeds the user is following.
/// Is updated by information received from the server.
pub struct RssDisplay {
    feeds_info: HashMap<Url, FeedInfo>,
    feeds_display: FeedListDisplay,
    /// Entries we have recieved from the server, based on the selection in the feeds_display.
    feed_entries: Vec<DisplayFeedEntry>,
    /// How many feed entries we have requested last request.
    requested_entry_amount: usize,
    /// How many feed entries are available on the server.
    available_entry_amount: usize,
    /// Whether or not to request feed entries that have already been read.
    show_unread_entries: bool,
    /// Whether to show the side panel with the feed list or not.
    open_sidepanel: bool,
    /// Previous size of the web page
    /// used to determine when the size changes.
    previous_page_size: Vec2,
}

impl RssDisplay {
    pub fn new(ctx: &egui::Context) -> Self {
        let page_size = ctx.screen_rect().size();
        let open_sidepanel = page_size.x >= SIDEPANEL_COLLAPSE_WIDTH;

        RssDisplay {
            feeds_info: HashMap::new(),
            feeds_display: FeedListDisplay::new(),
            feed_entries: vec![],
            requested_entry_amount: DEFAULT_ENTRY_REQUEST_AMOUNT,
            available_entry_amount: 0,
            show_unread_entries: false,
            open_sidepanel,
            previous_page_size: page_size,
        }
    }

    pub fn show_feeds_button(&mut self, ui: &mut Ui) {
        ui.toggle_value(&mut self.open_sidepanel, "Feeds");
    }

    pub fn handle_popups(&mut self, ctx: &egui::Context, requests: &mut Requests) {
        let response = self.feeds_display.handle_popups(ctx, requests);

        match response {
            FeedListPopupResponse::None => todo!(),
            FeedListPopupResponse::FeedInfoEdited(url, new_info) => {
                if let Some(feed) = self.feeds_info.get_mut(&url) {
                    *feed = new_info;
                }

                self.feeds_display.update_feeds_info(&self.feeds_info);
            }
            FeedListPopupResponse::FeedAdded => requests.new_request_with_json_body(
                ApiEndpoint::Feeds,
                FeedsRequest {
                    filter: self.feeds_display.current_selection(),
                    entry_filter: if self.show_unread_entries {
                        EntryTypeFilter::All
                    } else {
                        EntryTypeFilter::Unread
                    },
                    amount: self.requested_entry_amount,
                    additional_action: AdditionalAction::IncludeFeedsInfo,
                },
            ),
        }
    }

    pub fn show_feed_list(&mut self, ctx: &egui::Context, requests: &mut Requests) {
        let page_size = ctx.screen_rect().size();

        if page_size.x < SIDEPANEL_COLLAPSE_WIDTH
            && self.previous_page_size.x >= SIDEPANEL_COLLAPSE_WIDTH
        {
            // Went below the collapse size.
            self.open_sidepanel = false;
        } else if page_size.x >= SIDEPANEL_COLLAPSE_WIDTH
            && self.previous_page_size.x < SIDEPANEL_COLLAPSE_WIDTH
        {
            // Went above the collapse size.
            self.open_sidepanel = true;
        }

        self.previous_page_size = page_size;

        if !self.open_sidepanel {
            return;
        }

        egui::SidePanel::left("side-panel").show(ctx, |ui| {
            let last_show_read_entries = self.show_unread_entries;
            ui.checkbox(&mut self.show_unread_entries, "Show read entries");

            if last_show_read_entries != self.show_unread_entries {
                requests.new_request_with_json_body(
                    ApiEndpoint::Feeds,
                    FeedsRequest {
                        filter: self.feeds_display.current_selection(),
                        entry_filter: if self.show_unread_entries {
                            EntryTypeFilter::All
                        } else {
                            EntryTypeFilter::Unread
                        },
                        amount: self.requested_entry_amount,
                        additional_action: AdditionalAction::None,
                    },
                )
            }

            if ui.button("Update all feeds").clicked() {
                requests.new_request_with_json_body(
                    ApiEndpoint::Feeds,
                    FeedsRequest {
                        filter: self.feeds_display.current_selection(),
                        entry_filter: if self.show_unread_entries {
                            EntryTypeFilter::All
                        } else {
                            EntryTypeFilter::Unread
                        },
                        amount: self.requested_entry_amount,
                        additional_action: AdditionalAction::UpdateFeeds,
                    },
                )
            }

            match self.feeds_display.show(ui) {
                FeedListDisplayResponse::None => {} // Nothing to do
                FeedListDisplayResponse::SelectionChanged => {
                    self.on_feed_selection_changed(requests);
                }
            }
        });
    }

    pub fn show_entry_amount_display(&mut self, ui: &mut Ui, requests: &mut Requests) {
        if self.available_entry_amount > self.feed_entries.len() {
            // We only display the "request more" button if there is actually more to request.
            if ui
                .button(format!(
                    "{}/{} request more",
                    self.feed_entries.len(),
                    self.available_entry_amount
                ))
                .clicked()
            {
                requests.new_request_with_json_body(
                    ApiEndpoint::Feeds,
                    FeedsRequest {
                        filter: self.feeds_display.current_selection(),
                        entry_filter: if self.show_unread_entries {
                            EntryTypeFilter::All
                        } else {
                            EntryTypeFilter::Unread
                        },
                        amount: self.feed_entries.len() + DEFAULT_ENTRY_REQUEST_AMOUNT,
                        additional_action: AdditionalAction::None,
                    },
                )
            }
        }
    }

    /// Request the first [`DEFAULT_ENTRY_REQUEST_AMOUNT`] entries of the selected feeds.
    fn on_feed_selection_changed(&mut self, requests: &mut Requests) {
        self.feed_entries.clear();

        requests.new_request_with_json_body(
            ApiEndpoint::Feeds,
            FeedsRequest {
                filter: self.feeds_display.current_selection(),
                entry_filter: if self.show_unread_entries {
                    EntryTypeFilter::All
                } else {
                    EntryTypeFilter::Unread
                },
                amount: DEFAULT_ENTRY_REQUEST_AMOUNT,
                additional_action: AdditionalAction::None,
            },
        )
    }

    pub fn show_feed_entries(&mut self, ui: &mut Ui, requests: &mut Requests) {
        if requests.has_request(ApiEndpoint::Feeds) {
            if let Some(response) = requests.ready(ApiEndpoint::Feeds) {
                // TODO (Wybe 2022-07-16): Handle errors
                // TODO (Wybe 2022-07-18): Reduce nesting
                if let Response::Ok(body) = response {
                    if let Ok(feeds_response) = serde_json::from_str::<FeedsResponse>(&body) {
                        if let Some(feeds_info) = feeds_response.feeds_info {
                            self.feeds_info = feeds_info;
                        }

                        self.available_entry_amount = feeds_response.total_available;
                        self.feed_entries.clear();

                        for entry in feeds_response.feed_entries {
                            let feed_name = self
                                .feeds_info
                                .get(&entry.feed_url)
                                .map(|feed| feed.name.as_str())
                                .unwrap_or("");

                            self.feed_entries
                                .push(DisplayFeedEntry::new(&entry, feed_name));
                        }
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
                    if let Some((index, _)) = self
                        .feed_entries
                        .iter_mut()
                        .enumerate()
                        .find(|(_, entry)| entry.key == response.entry_key)
                        .take()
                    {
                        // If we are not displaying unread entries, we should remove it. Otherwise update it.
                        if !self.show_unread_entries && !response.read {
                            self.feed_entries.remove(index);
                            // If we have removed the entry from this view, there will be one less entry available from the server
                            // if we were to re-request the view.
                            self.available_entry_amount =
                                self.available_entry_amount.saturating_sub(1);
                        } else {
                            self.feed_entries[index].read = response.read;
                        }
                    }
                }
            }
        }

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        let unread_entry_text_color = ui.ctx().style().visuals.strong_text_color();

        let mut set_entry_read_request = None;

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show_rows(ui, row_height, self.feed_entries.len(), |ui, row_range| {
                egui::Grid::new("feed-grid")
                    .striped(true)
                    .num_columns(5)
                    .start_row(row_range.start)
                    .show(ui, |ui| {
                        for entry in self
                            .feed_entries
                            .iter()
                            .skip(row_range.start)
                            //TODO (Wybe 2022-07-18): Vertical scroll bar changes size sometimes during scrolling, why?
                            .take(row_range.end - row_range.start)
                        {
                            let unread = !entry.read;

                            let mut mark_read = !unread;
                            ui.checkbox(
                                &mut mark_read,
                                highlighted_text(
                                    &entry.display_title,
                                    unread,
                                    unread_entry_text_color,
                                ),
                            );

                            if mark_read == unread {
                                // User wants to mark this entry as read or unread.
                                set_entry_read_request = Some(SetEntryReadRequestAndResponse {
                                    feed_url: entry.feed_url.clone(),
                                    entry_key: entry.key.clone(),
                                    read: mark_read,
                                });
                            }

                            ui.label(highlighted_text(
                                &entry.pub_date_string,
                                unread,
                                unread_entry_text_color,
                            ));

                            ui.label(highlighted_text(
                                &entry.feed_name,
                                unread,
                                unread_entry_text_color,
                            ));

                            if let Some(link) = &entry.link {
                                ui.add(NewTabHyperlink::from_label_and_url("Open", link));

                                if !entry.read {
                                    // Item not read, so we add an option to open and mark it "read" at the same time.
                                    if ui
                                        .add(NewTabHyperlink::from_label_and_url(
                                            "Open mark read",
                                            link,
                                        ))
                                        .clicked()
                                        && !entry.read
                                    {
                                        // User wants to mark this entry as read.
                                        set_entry_read_request =
                                            Some(SetEntryReadRequestAndResponse {
                                                feed_url: entry.feed_url.clone(),
                                                entry_key: entry.key.clone(),
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
            });

        if let Some(request) = set_entry_read_request {
            requests.new_request_with_json_body(ApiEndpoint::SetEntryRead, request);
        }
    }

    /// Call this after the user has logged in.
    pub fn on_login(&self, requests: &mut Requests) {
        // Do the first feeds request.
        // Because we have just logged in, we request to include the feeds info.
        requests.new_request_with_json_body(
            ApiEndpoint::Feeds,
            FeedsRequest {
                filter: self.feeds_display.current_selection(),
                entry_filter: if self.show_unread_entries {
                    EntryTypeFilter::All
                } else {
                    EntryTypeFilter::Unread
                },
                amount: DEFAULT_ENTRY_REQUEST_AMOUNT,
                additional_action: AdditionalAction::IncludeFeedsInfo,
            },
        );
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
#[derive(Debug, Clone)]
struct DisplayFeedEntry {
    /// The feed title, but formatted for display.
    display_title: String,
    /// Key to use when sending update requests to the server, such as marking the entry as read.
    key: EntryKey,
    /// Name of the feed this entry belongs to.
    feed_name: String,
    feed_url: Url,
    link: Option<Url>,
    pub_date_string: String,
    read: bool,
}

impl DisplayFeedEntry {
    fn new(entry: &ComFeedEntry, feed_name: &str) -> Self {
        let display_title = cut_middle_of_string_if_too_long(&entry.title, 60);
        let feed_title = cut_middle_of_string_if_too_long(feed_name, 40);

        DisplayFeedEntry {
            display_title,
            key: entry.key.clone(),
            feed_name: feed_title,
            feed_url: entry.feed_url.clone(),
            link: entry.link.clone(),
            pub_date_string: entry
                .pub_date
                .with_timezone(&Local)
                .format("%Y-%m-%d")
                .to_string(),
            read: entry.read,
        }
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
