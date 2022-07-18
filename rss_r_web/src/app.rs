use crate::login::Login;
use crate::requests::{ApiEndpoint, Requests, Response};
use crate::rss_collection::RssCollection;
use eframe::Frame;
use egui::{Align2, Context, Ui, Vec2, Visuals};
use log::info;

pub struct RssApp {
    // TODO (Wybe 2022-07-11): Store config server side? And retrieve on log-in?
    config: Config,
    requests: Requests,
    login_view: Option<Login>,
    rss_collection: RssCollection,
    display_string: String,
}

impl RssApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config: Config = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let visuals = if config.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        cc.egui_ctx.set_visuals(visuals);

        RssApp {
            config,
            // TODO (Wybe 2022-07-10): Maybe make some kind of page system, where you can switch between pages, and don't need to keep each page in a different variable.
            requests: Requests::new(cc.egui_ctx.clone()),
            login_view: Some(Login::default()),
            rss_collection: RssCollection::new(),
            display_string: "".to_string(),
        }
    }
}

impl eframe::App for RssApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Update any outstanding http requests.
        self.requests.poll();

        if !self.requests.is_authenticated() && self.login_view.is_none() {
            // No longer authenticated. Back to login view.
            self.login_view = Some(Login::default());
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(dark_mode) = global_dark_light_mode_switch(ui) {
                    info!("Dark mode: {dark_mode}");
                    self.config.dark_mode = dark_mode;
                }

                ui.separator();

                if self.requests.has_request(ApiEndpoint::Logout) {
                    if self.requests.ready(ApiEndpoint::Logout).is_some() {
                        info!("Logged out");
                        self.requests.set_authenticated(false);
                        self.login_view = Some(Login::default());
                    } else {
                        ui.spinner();
                    }
                } else if self.login_view.is_none() && ui.button("Log out").clicked() {
                    self.requests.new_empty_request(ApiEndpoint::Logout)
                }
            });
        });

        if self.login_view.is_none() {
            egui::SidePanel::left("side-panel").show(ctx, |ui| {
                self.rss_collection.show_list(ctx, ui, &mut self.requests)
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.login_view.is_none() {
                self.rss_collection
                    .show_feed_entries(ui, &mut self.requests);
            }
        });

        let mut logged_in = false;
        if let Some(login) = &mut self.login_view {
            egui::Window::new("Login")
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    logged_in = login.show(ui, &mut self.requests);
                });
        }

        if logged_in {
            self.requests.set_authenticated(true);
            self.login_view = None;
            self.rss_collection = RssCollection::new();

            // Request the available feeds.
            self.requests.new_empty_request(ApiEndpoint::ListFeeds);
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        info!("Saving");
        eframe::set_value(storage, eframe::APP_KEY, &self.config);
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
struct Config {
    dark_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config { dark_mode: true }
    }
}

/// Returns whether the user selected dark mode.
fn global_dark_light_mode_switch(ui: &mut Ui) -> Option<bool> {
    let style = (*ui.ctx().style()).clone();
    let new_visuals = style.visuals.light_dark_small_toggle_button(ui);

    if let Some(visuals) = new_visuals {
        let result = Some(visuals.dark_mode);
        ui.ctx().set_visuals(visuals);
        return result;
    }
    None
}
