use crate::add_feed_popup::{AddFeedPopup, AddFeedPopupResponse};
use crate::edit_feed_popup::{EditFeedPopup, EditFeedPopupResponse};
use crate::requests::Requests;
use egui::collapsing_header::CollapsingState;
use egui::{RichText, Ui};
use rss_com_lib::message_body::FeedsFilter;
use rss_com_lib::rss_feed::FeedInfo;
use rss_com_lib::Url;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Default)]
pub struct FeedListDisplay {
    /// A copy of all the known feeds, but in a layout suited for quick display.
    /// tags -> Feeds.
    feed_tags: BTreeMap<String, Vec<(Url, FeedInfo)>>,
    feeds_without_tags: Vec<(Url, FeedInfo)>,
    /// A copy of all known tags. For quick access.
    known_tags: HashSet<String>,
    selection: FeedsFilter,
    add_feed_popup: Option<AddFeedPopup>,
    edit_feed_popup: Option<EditFeedPopup>,
}

impl FeedListDisplay {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update_feeds_info(&mut self, new_feeds: &HashMap<Url, FeedInfo>) {
        let mut feeds_by_tag: BTreeMap<String, Vec<(Url, FeedInfo)>> = BTreeMap::new();
        self.feeds_without_tags = Vec::new();
        self.known_tags = HashSet::new();

        // Collect all the feeds per tag.
        for (url, info) in new_feeds.iter() {
            for tag in info.tags.iter() {
                feeds_by_tag
                    .entry(tag.clone())
                    .and_modify(|by_tag| by_tag.push((url.clone(), info.clone())))
                    .or_insert_with(|| vec![(url.clone(), info.clone())]);

                self.known_tags.insert(tag.clone());
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
        match &self.selection {
            FeedsFilter::All => {
                // Nothing to do.
            }
            FeedsFilter::Tag(tag) => {
                if !feeds_by_tag.contains_key(tag) {
                    // Tag has disappeared.
                    self.selection = FeedsFilter::All;
                }
            }
            FeedsFilter::Single(_) => {
                // todo: Check if the url is still there.
            }
        }

        self.feed_tags = feeds_by_tag;
    }

    pub fn current_selection(&self) -> FeedsFilter {
        self.selection.clone()
    }

    pub fn show(&mut self, ui: &mut Ui) -> FeedListDisplayResponse {
        let mut response = FeedListDisplayResponse::None;

        if ui.button("➕ Add feed").clicked() && self.add_feed_popup.is_none() {
            self.add_feed_popup = Some(AddFeedPopup::new(self.known_tags.clone()));
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if selectable_value(ui, self.selection == FeedsFilter::All, "All feeds") {
                self.selection = FeedsFilter::All;
                response = FeedListDisplayResponse::SelectionChanged;
            }

            if !self.feeds_without_tags.is_empty() {
                ui.collapsing("Untagged", |ui| {
                    for (url, info) in self.feeds_without_tags.iter() {
                        feed_info_display(
                            ui,
                            url,
                            info,
                            &mut response,
                            &mut self.selection,
                            &mut self.edit_feed_popup,
                            &self.known_tags,
                        );
                    }
                });
            }

            for (tag, feeds) in self.feed_tags.iter() {
                let collapse_id = ui.make_persistent_id(tag);
                CollapsingState::load_with_default_open(ui.ctx(), collapse_id, false)
                    .show_header(ui, |ui| {
                        let tag_selected = match &self.selection {
                            FeedsFilter::Tag(selected_tag) => selected_tag == tag,
                            _ => false,
                        };

                        if selectable_value(ui, tag_selected, tag) {
                            self.selection = FeedsFilter::Tag(tag.clone());

                            response = FeedListDisplayResponse::SelectionChanged;
                        }
                    })
                    .body(|ui| {
                        for (url, info) in feeds {
                            feed_info_display(
                                ui,
                                url,
                                info,
                                &mut response,
                                &mut self.selection,
                                &mut self.edit_feed_popup,
                                &self.known_tags,
                            );
                        }
                    });
            }
        });

        response
    }

    pub fn handle_popups(
        &mut self,
        ctx: &egui::Context,
        requests: &mut Requests,
    ) -> FeedListPopupResponse {
        let mut response = FeedListPopupResponse::None;

        // Handle "Add feed" popup
        if let Some(popup) = &mut self.add_feed_popup {
            match popup.show(ctx, requests) {
                AddFeedPopupResponse::None => {} // Nothing to do.
                AddFeedPopupResponse::ClosePopup => {
                    self.add_feed_popup = None;
                }
                AddFeedPopupResponse::FeedAdded => {
                    self.add_feed_popup = None;
                    response = FeedListPopupResponse::FeedAdded;
                }
            }
        }

        // Handle "edit feed info" popup.
        if let Some(popup) = &mut self.edit_feed_popup {
            match popup.show(ctx, requests) {
                EditFeedPopupResponse::None => {} // Nothing to do.
                EditFeedPopupResponse::ClosePopup => {
                    self.edit_feed_popup = None;
                }
                EditFeedPopupResponse::FeedInfoEdited(url, new_info) => {
                    // Edit was a success. Close the popup.
                    self.edit_feed_popup = None;

                    response = FeedListPopupResponse::FeedInfoEdited(url, new_info);
                }
            }
        }

        response
    }
}

fn feed_info_display(
    ui: &mut Ui,
    feed_url: &Url,
    info: &FeedInfo,
    response: &mut FeedListDisplayResponse,
    selection: &mut FeedsFilter,
    edit_feed_popup: &mut Option<EditFeedPopup>,
    known_tags: &HashSet<String>,
) {
    let selected = match selection {
        FeedsFilter::Single(selected_url) => selected_url == feed_url,
        _ => false,
    };

    ui.horizontal(|ui| {
        match &info.last_update_result {
            Ok(()) => {
                ui.label("-");
            }
            Err(message) => {
                ui.label(RichText::new("?").color(ui.visuals().error_fg_color))
                    .on_hover_text(message);
            }
        }

        ui.horizontal_wrapped(|ui| {
            if selectable_value(ui, selected, &info.name) {
                *selection = FeedsFilter::Single(feed_url.clone());
                *response = FeedListDisplayResponse::SelectionChanged;
            }

            // Only show the edit buton if the feed is selected.
            if selected && ui.button("Edit").clicked() && edit_feed_popup.is_none() {
                *edit_feed_popup = Some(EditFeedPopup::new(
                    feed_url.clone(),
                    info.clone(),
                    known_tags.clone(),
                ));
            }
        });
    });
}

pub enum FeedListDisplayResponse {
    None,
    SelectionChanged,
}

pub enum FeedListPopupResponse {
    None,
    FeedInfoEdited(Url, FeedInfo),
    FeedAdded,
}

/// A selectable value that will return true if it has been selected by the user.
fn selectable_value(ui: &mut Ui, mut selected: bool, text: &str) -> bool {
    ui.toggle_value(&mut selected, text).clicked()
}
