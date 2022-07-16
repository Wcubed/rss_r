use crate::login::State::LoggedIn;
use crate::requests::{ApiEndpoint, HttpStatus, Requests, Response};
use egui::{Button, Layout, TextEdit, Ui};
use log::info;
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};

#[derive(Default)]
pub struct Login {
    username: String,
    password: String,
    state: State,
    show_invalid_user_or_password_message: bool,
}

impl Login {
    /// Returns `true` if the login is successful.
    pub fn show(&mut self, ui: &mut Ui, requests: &mut Requests) -> bool {
        match self.state {
            State::New => {
                // Test whether the identity cookie is still valid, by performing a login
                // request without username and password.
                requests.new_empty_request(ApiEndpoint::TestAuthCookie);
                self.state = State::TryIdentityCookieLogin;

                false
            }
            State::TryIdentityCookieLogin => {
                if requests.has_request(ApiEndpoint::TestAuthCookie) {
                    if let Some(response) = requests.ready(ApiEndpoint::TestAuthCookie) {
                        if let Response::Ok(_) = response {
                            // Identity cookie login OK.
                            info!("Logged in with identity cookie");
                            self.state = LoggedIn
                        } else {
                            self.state = State::UsernameAndPasswordLogin;
                        }
                    } else {
                        ui.spinner();
                    }
                } else {
                    self.state = State::UsernameAndPasswordLogin;
                }
                false
            }
            State::UsernameAndPasswordLogin => {
                self.show_login_fields(ui, requests);

                if requests.has_request(ApiEndpoint::Login) {
                    let response = requests.ready(ApiEndpoint::Login);
                    if let Some(Response::Ok(_)) = response {
                        info!("Logged in with password");
                        self.show_invalid_user_or_password_message = false;
                        self.state = State::LoggedIn;
                    } else if let Some(Response::NotOk(status)) = response {
                        if status == HttpStatus::Unauthorized {
                            self.show_invalid_user_or_password_message = true;
                        } else {
                            // TODO (Wybe 2022-07-12): Show some kind of error message?
                        }
                    } else {
                        self.show_invalid_user_or_password_message = false;
                        ui.spinner();
                    }
                } else if self.show_invalid_user_or_password_message {
                    ui.colored_label(egui::Color32::RED, "Invalid username or password");
                }

                false
            }
            State::LoggedIn => {
                // Request the available feeds.
                requests.new_empty_request(ApiEndpoint::ListFeeds);

                true
            }
        }
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
                // TODO (Wybe 2022-07-12): Hash the password before sending it. So a user with a 4GB password won't overload the connection (and ddos the server).
                req.headers
                    .insert(PASSWORD_HEADER.to_string(), self.password.to_string());
            });

            self.username = String::new();
            self.password = String::new();
        }
    }
}

enum State {
    New,
    TryIdentityCookieLogin,
    UsernameAndPasswordLogin,
    LoggedIn,
}

impl Default for State {
    fn default() -> Self {
        State::New
    }
}
