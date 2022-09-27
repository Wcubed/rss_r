use crate::requests::{ApiEndpoint, Requests, Response};
use crate::{POPUP_ALIGN, POPUP_OFFSET};
use eframe::emath::{Align2, Vec2};
use egui::{Context, TextEdit, Ui};
use rss_com_lib::message_body::SetFeedInfoRequestAndResponse;
use rss_com_lib::rss_feed::FeedInfo;
use rss_com_lib::Url;
use std::collections::HashSet;

pub struct EditFeedPopup {
    feed_url: Url,
    feed_info: FeedInfo,
    tag_selector: TagSelector,
}

impl EditFeedPopup {
    pub fn new(feed_url: Url, feed_info: FeedInfo) -> Self {
        let tag_selector = TagSelector::new(&feed_info.tags);

        Self {
            feed_url,
            feed_info,
            tag_selector,
        }
    }

    pub fn show(&mut self, ctx: &Context, requests: &mut Requests) -> EditFeedPopupResponse {
        let mut response = EditFeedPopupResponse::None;
        let mut is_open = true;

        egui::Window::new("Edit feed")
            .open(&mut is_open)
            .anchor(POPUP_ALIGN, POPUP_OFFSET)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading(&self.feed_info.name);

                self.tag_selector.show(ui);

                // TODO (Wybe 2022-09-25): Add an api to edit a feed's info.
                if ui.button("Save").clicked() {
                    self.feed_info.tags = self.tag_selector.get_selected_tags();

                    requests.new_request_with_json_body(
                        ApiEndpoint::SetFeedInfo,
                        SetFeedInfoRequestAndResponse {
                            feed_url: self.feed_url.clone(),
                            info: self.feed_info.clone(),
                        },
                    )
                }

                if requests.has_request(ApiEndpoint::SetFeedInfo) {
                    // TODO (Wybe 2022-09-27): Add error handling.
                    if let Some(Response::Ok(body)) = requests.ready(ApiEndpoint::SetFeedInfo) {
                        if let Ok(feeds_response) =
                            serde_json::from_str::<SetFeedInfoRequestAndResponse>(&body)
                        {
                            // Success.
                            response = EditFeedPopupResponse::FeedInfoEdited(
                                feeds_response.feed_url,
                                feeds_response.info,
                            );
                        }
                    } else {
                        ui.spinner();
                    }
                }
            });

        if response == EditFeedPopupResponse::None && !is_open {
            response = EditFeedPopupResponse::ClosePopup;
        }

        response
    }
}

#[derive(Eq, PartialEq)]
pub enum EditFeedPopupResponse {
    /// Nothing to do.
    None,
    /// User wants to close the popup. No new feeds.
    ClosePopup,
    /// Info was edited. Contains the url of the edited feed, and the new info.
    FeedInfoEdited(Url, FeedInfo),
}

pub struct TagSelector {
    /// List of tags and whether they are selected for this feed.
    tags: Vec<(String, bool)>,
    new_tag: String,
}

impl TagSelector {
    pub fn new(tags: &HashSet<String>) -> Self {
        // TODO (Wybe 2022-09-25): Add all known tags from all feeds as options to the tag list.
        TagSelector {
            tags: tags.iter().cloned().map(|tag| (tag, true)).collect(),
            new_tag: String::new(),
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.heading("Tags");

        for (tag, selected) in self.tags.iter_mut() {
            // TODO (Wybe 2022-09-25): We should be able to show the tag without cloning the text.
            ui.checkbox(selected, tag.clone());
        }

        ui.horizontal(|ui| {
            let edit_response = TextEdit::singleline(&mut self.new_tag)
                .hint_text("New tag")
                .show(ui)
                .response;

            let add_button_clicked = ui.button("+").clicked();

            if add_button_clicked
                || (edit_response.lost_focus() && ui.input().key_pressed(egui::Key::Enter))
            {
                self.tags.push((self.new_tag.clone(), true));
                self.new_tag = String::new();
            }
        });

        ui.separator();
    }

    pub fn get_selected_tags(&self) -> HashSet<String> {
        self.tags
            .iter()
            .filter_map(|(tag, selected)| if *selected { Some(tag.clone()) } else { None })
            .collect()
    }
}
