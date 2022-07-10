use egui::{TextEdit, Ui};
use log::info;

#[derive(Default)]
pub struct Login {
    username: String,
    password: String,
}

impl Login {
    pub fn show(&mut self, ui: &mut Ui) {
        TextEdit::singleline(&mut self.username)
            .hint_text("Username")
            .show(ui);
        let response = TextEdit::singleline(&mut self.password)
            .hint_text("Password")
            .password(true)
            .show(ui)
            .response;

        let log_in_clicked = ui.button("Log in").clicked();

        if log_in_clicked || (response.lost_focus() && ui.input().key_pressed(egui::Key::Enter)) {
            // TODO (Wybe 2022-07-10): Do some kind of request for logging in.
            info!("Logging in");
            self.username = String::new();
            self.password = String::new();
        }
    }
}
