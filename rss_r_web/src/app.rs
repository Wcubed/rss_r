use crate::login::Login;
use eframe::Frame;
use egui::{Align2, Context, Ui, Vec2, Visuals};
use log::info;
use poll_promise::Promise;

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RssApp {
    config: Config,
    #[serde(skip)]
    login_view: Option<Login>,
    /// TODO (Wybe 2022-07-10): Add a convenience struct of some kind for promises.
    #[serde(skip)]
    test_promise: Option<Promise<ehttp::Result<String>>>,
    #[serde(skip)]
    logout_promise: Option<Promise<ehttp::Result<()>>>,
}

impl RssApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: RssApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        // Always start at the login page.
        // TODO (Wybe 2022-07-10): Maybe make some kind of page system, where you can switch between pages, and don't need to keep each page in a different variable.
        app.login_view = Some(Login::default());

        let visuals = if app.config.dark_mode {
            Visuals::dark()
        } else {
            Visuals::light()
        };
        cc.egui_ctx.set_visuals(visuals);

        app
    }
}

impl eframe::App for RssApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(dark_mode) = global_dark_light_mode_switch(ui) {
                    info!("Dark mode: {dark_mode}");
                    self.config.dark_mode = dark_mode;
                }

                ui.separator();

                if self.login_view.is_none() {
                    if ui.button("Log out").clicked() {
                        let (sender, promise) = Promise::new();
                        let request = ehttp::Request::get("../api/logout");

                        let ctx = ctx.clone();
                        ehttp::fetch(request, move |response| {
                            ctx.request_repaint(); // Wake up UI thread.

                            let result = response.map(|response| ());

                            sender.send(result);
                        });
                        self.logout_promise = Some(promise);
                    }
                }

                if let Some(promise) = &self.logout_promise {
                    if let Some(result) = promise.ready() {
                        match result {
                            Ok(()) => {
                                info!("Logged out");
                            }
                            Err(error) => {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    if error.is_empty() { "Error" } else { error },
                                );
                            }
                        }
                    } else {
                        ui.spinner();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.login_view.is_none() {
                ui.heading("Hello World!");

                if (ui.button("Do http request")).clicked() {
                    // Testing http request code.
                    let (sender, promise) = Promise::new();
                    let request = ehttp::Request::get("../api/");

                    let ctx = ctx.clone();
                    ehttp::fetch(request, move |response| {
                        ctx.request_repaint(); // Wake up UI thread.

                        let result = response
                            .map(|response| response.text().unwrap_or_default().to_string());

                        sender.send(result);
                    });
                    self.test_promise = Some(promise);
                }

                if let Some(promise) = &self.test_promise {
                    if let Some(result) = promise.ready() {
                        match result {
                            Ok(string) => {
                                ui.label(string);
                            }
                            Err(error) => {
                                ui.colored_label(
                                    egui::Color32::RED,
                                    if error.is_empty() { "Error" } else { error },
                                );
                            }
                        }
                    } else {
                        ui.spinner();
                    }
                }
            }
        });

        let mut logged_in = false;
        if let Some(login) = &mut self.login_view {
            egui::Window::new("Login")
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    logged_in = login.show(ctx, ui);
                });
        }

        if logged_in {
            self.login_view = None;
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
