use egui::{Context, TextEdit, Ui};
use log::info;
use poll_promise::Promise;
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};

const LOGIN_URL: &str = "../api/login";

#[derive(Default)]
pub struct Login {
    username: String,
    password: String,
    login_promise: Option<Promise<ehttp::Result<ehttp::Response>>>,
}

impl Login {
    /// Returns `true` if the login is successful.
    pub fn show(&mut self, ctx: &Context, ui: &mut Ui) -> bool {
        self.show_login_fields(ctx, ui);

        let mut logged_in = false;

        if let Some(promise) = &self.login_promise {
            if let Some(result) = promise.ready() {
                match result {
                    Ok(response) => {
                        if response.ok {
                            logged_in = true;
                        } else {
                            ui.colored_label(egui::Color32::RED, "Invalid username or password");
                        }
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

        logged_in
    }

    fn show_login_fields(&mut self, ctx: &Context, ui: &mut Ui) {
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
            let (sender, promise) = Promise::new();
            let mut request = ehttp::Request::get(LOGIN_URL);

            // TODO (Wybe 2022-07-10): Should the id and password be base64 encoded?
            request
                .headers
                .insert(USER_ID_HEADER.to_string(), self.username.to_string());
            request
                .headers
                .insert(PASSWORD_HEADER.to_string(), self.password.to_string());

            let ctx = ctx.clone();
            ehttp::fetch(request, move |response| {
                ctx.request_repaint(); // Wake up UI thread.

                sender.send(response);
            });
            self.login_promise = Some(promise);

            self.username = String::new();
            self.password = String::new();
        }
    }
}
