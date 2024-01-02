use crate::login::LoginView;
use crate::requests::{ApiEndpoint, Requests};
use crate::rss_collection::RssCollection;
use eframe::Frame;
use egui::{Align2, Context, Ui, Vec2, Visuals};
use log::info;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct RssApp {
    // TODO (Wybe 2022-07-11): Store config server side? And retrieve on log-in?
    config: Config,
    requests: Requests,
    active_view: ActiveView,
    version_string: String,
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
            active_view: ActiveView::Login(LoginView::default()),
            version_string: format!("v{}", VERSION),
        }
    }
}

impl eframe::App for RssApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Update any outstanding http requests.
        self.requests.poll();

        let at_login_view = matches!(self.active_view, ActiveView::Login(_));

        if !self.requests.is_authenticated() && !at_login_view {
            // No longer authenticated. Back to login view.
            self.active_view = ActiveView::Login(LoginView::default());
        }

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let ActiveView::RssCollection(collection) = &mut self.active_view {
                    collection.show_feeds_button(ui);
                }

                if self.requests.has_request(ApiEndpoint::Logout) {
                    if self.requests.ready(ApiEndpoint::Logout).is_some() {
                        info!("Logged out");
                        self.requests.set_authenticated(false);
                        self.active_view = ActiveView::Login(LoginView::default());
                    } else {
                        ui.spinner();
                    }
                } else if !at_login_view && ui.button("Log out").clicked() {
                    self.requests.new_request_without_body(ApiEndpoint::Logout)
                }

                ui.separator();

                if let Some(dark_mode) = global_dark_light_mode_switch(ui) {
                    info!("Dark mode: {dark_mode}");
                    self.config.dark_mode = dark_mode;
                }

                ui.label(&self.version_string);
            });
        });

        if let ActiveView::RssCollection(collection) = &mut self.active_view {
            collection.show_feed_list(ctx, &mut self.requests);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let ActiveView::RssCollection(collection) = &mut self.active_view {
                collection.show_feed_entries(ui, &mut self.requests);
            }
        });

        let mut logged_in = false;
        if let ActiveView::Login(login) = &mut self.active_view {
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
            self.active_view = ActiveView::RssCollection(Box::new(RssCollection::new()));

            // Request the available feeds.
            self.requests
                .new_request_without_body(ApiEndpoint::ListFeeds);
        }
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        info!("Saving");
        eframe::set_value(storage, eframe::APP_KEY, &self.config);
    }
}

enum ActiveView {
    Login(LoginView),
    RssCollection(Box<RssCollection>),
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
