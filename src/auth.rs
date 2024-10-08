use crate::persistence::SaveInRonFile;
use crate::users::{UserId, UserRequestInfo, Users};
use crate::{Authenticated, UserInfo};
use actix_identity::Identity;
use actix_web::dev::ServiceRequest;
use actix_web::{post, web, HttpMessage, HttpRequest, HttpResponse, Responder};
use log::{info, warn};
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

pub const AUTH_COOKIE_NAME: &str = "auth_id";

#[derive(Serialize, Deserialize)]
pub struct AuthData {
    users: RwLock<Users>,
}

impl AuthData {
    /// TODO (Wybe 2022-07-12): Take encrypted password instead of raw string.
    /// TODO (Wybe 2022-07-12): Have a maximum to the user name length?
    /// TODO (Wybe 2022-07-12): Instead of the username to log-in, use an email address?
    /// TODO (Wybe 2022-07-12): Generate a user id, instead of taking one.
    pub fn new_user(&mut self, id: UserId, user_info: UserInfo) {
        let mut users = self.users.write().unwrap();
        users.insert(id, user_info);
    }

    /// TODO (Wybe 2022-07-11): Implement storing session ids instead of user ids.
    /// TODO (Wybe 2022-07-12): Check whether this user is allowed to access this url.
    ///     Don't return 401 (unauthorized) but 403 (forbidden) to indicate that
    ///     "yes you are logged in, but no, you don't have rights to view this"
    pub fn authenticate_user_id(
        &self,
        identity: Identity,
        _request: &ServiceRequest,
    ) -> Option<AuthenticationResult> {
        let users = self.users.read().unwrap();

        if let Ok(Some(id)) = identity
            .id()
            .map(|user_id_string| UserId::from_str(&user_id_string))
        {
            users.get(&id).map(|info| AuthenticationResult {
                user: info.get_request_info(id),
            })
        } else {
            None
        }
    }

    pub fn validate_password(&self, user_name: &str, password: &str) -> Option<UserId> {
        let users = self.users.read().unwrap();

        if let Some((&id, info)) = users.iter().find(|(_, info)| info.name == user_name) {
            if info.password == password {
                Some(id)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Default for AuthData {
    fn default() -> Self {
        let mut auth = Self {
            users: RwLock::new(Default::default()),
        };

        // TODO (Wybe 2022-07-12): Have some way of creating users.
        auth.new_user(
            UserId(1),
            UserInfo {
                name: "test".to_string(),
                password: "testing".to_string(),
            },
        );

        auth
    }
}

impl SaveInRonFile for AuthData {
    const FILE_NAME: &'static str = "auth.ron";
}

/// TODO (Wybe 2022-07-11): Add authentication info.
pub struct AuthenticationResult {
    user: UserRequestInfo,
}

impl AuthenticationResult {
    pub fn user_id(&self) -> &UserId {
        &self.user.id
    }

    pub fn user_name(&self) -> &str {
        &self.user.name
    }
}

/// Validates user identity cookie.
/// This could also be done by calling any other authenticated service, but having a dedicated
/// endpoint for it, that doesn't have any side effects, is neater.
#[post("/test_auth_cookie")]
pub async fn test_auth_cookie(auth: Authenticated) -> impl Responder {
    info!(
        "User `{}` connected with valid identity cookie",
        auth.user_name()
    );
    // The fact that the `Authenticated` struct was available means the cookie was valid.
    HttpResponse::Ok().finish()
}

/// Validates user id and password, and sets an identity cookie if they are valid.
#[post("/login")]
pub async fn login(req: HttpRequest, auth_data: web::Data<AuthData>) -> impl Responder {
    if let (Some(user_name), Some(password)) = (
        req.headers()
            .get(USER_ID_HEADER)
            .and_then(|id| id.to_str().ok()),
        req.headers()
            .get(PASSWORD_HEADER)
            .and_then(|pass| pass.to_str().ok()),
    ) {
        // TODO (Wybe 2022-07-10): Allow registering and remembering users and such.
        if let Some(user_id) = auth_data.validate_password(user_name, password) {
            // Login valid. Remember in the session that the user logged in.
            if let Err(error) = Identity::login(&req.extensions(), user_id.0.to_string()) {
                warn!(
                    "Something went wrong while trying to log in user with password `{}`: {}",
                    user_name, error
                );

                HttpResponse::Unauthorized().finish()
            } else {
                info!("User `{user_name}` logged in with password");

                HttpResponse::Ok().finish()
            }
        } else {
            HttpResponse::Unauthorized().finish()
        }
    } else {
        warn!(
            "Attempt to log in without {} and {} headers",
            USER_ID_HEADER, PASSWORD_HEADER
        );
        HttpResponse::Unauthorized().finish()
    }
}

/// Logs out the user by forgetting the authentication cookie.
#[post("/logout")]
pub async fn logout(id: Identity, auth: Authenticated) -> impl Responder {
    info!("Logging out `{}`", auth.user_name());

    id.logout();
    HttpResponse::Ok().finish()
}
