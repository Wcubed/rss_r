use eframe::{App, Frame};
use egui::{Context, Ui, Visuals};
use log::info;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct RssApp {
    dark_mode: bool,
}

impl Default for RssApp {
    fn default() -> Self {
        RssApp { dark_mode: true }
    }
}

impl RssApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app: RssApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        let visuals = if app.dark_mode {
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
                    self.dark_mode = dark_mode;
                }

                ui.separator();
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello World!");
        });
    }

    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        info!("Saving...");
        eframe::set_value(storage, eframe::APP_KEY, self);
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
