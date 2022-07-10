use eframe::Frame;
use egui::Context;

pub struct RssApp;

impl Default for RssApp {
    fn default() -> Self {
        Self
    }
}

impl RssApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for RssApp {
    fn update(&mut self, _ctx: &Context, _frame: &mut Frame) {}
}
