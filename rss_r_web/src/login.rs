use crate::requests::{ApiEndpoint, Requests, Response};
use egui::{Button, Context, TextEdit, Ui};
use log::info;
use poll_promise::Promise;
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};

#[derive(Default)]
pub struct Login {
    username: String,
    password: String,
    invalid_user_or_password: bool,
}

impl Login {
    /// Returns `true` if the login is successful.
    pub fn show(&mut self, ui: &mut Ui, requests: &mut Requests) -> bool {
        self.show_login_fields(ui, requests);

        let mut logged_in = false;

        if requests.has_request(ApiEndpoint::Login) {
            let response = requests.ready(ApiEndpoint::Login);
            if let Some(Response::Ok(_)) = response {
                info!("Logged in");
                self.invalid_user_or_password = false;
                logged_in = true;
            } else if let Some(Response::Unauthorized) = response {
                self.invalid_user_or_password = true;
            } else {
                self.invalid_user_or_password = false;
                ui.spinner();
            }
        } else if self.invalid_user_or_password {
            ui.colored_label(egui::Color32::RED, "Invalid username or password");
        }

        logged_in
    }

    fn show_login_fields(&mut self, ui: &mut Ui, requests: &mut Requests) {
        let login_interactive = !requests.has_request(ApiEndpoint::Login);

        TextEdit::singleline(&mut self.username)
            .hint_text("Username")
            .interactive(login_interactive)
            .show(ui);
        let response = TextEdit::singleline(&mut self.password)
            .hint_text("Password")
            .password(true)
            .interactive(login_interactive)
            .show(ui)
            .response;

        let log_in_clicked = ui
            .add_enabled(login_interactive, Button::new("Log in"))
            .clicked();

        if log_in_clicked || (response.lost_focus() && ui.input().key_pressed(egui::Key::Enter)) {
            requests.new_request(ApiEndpoint::Login, |req| {
                // TODO (Wybe 2022-07-10): Should the id and password be base64 encoded?
                req.headers
                    .insert(USER_ID_HEADER.to_string(), self.username.to_string());
                req.headers
                    .insert(PASSWORD_HEADER.to_string(), self.password.to_string());
            });

            self.username = String::new();
            self.password = String::new();
        }
    }
}
