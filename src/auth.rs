use crate::users::{UserId, UserRequestInfo, Users};
use crate::{Authenticated, UserInfo};
use actix_identity::Identity;
use actix_web::dev::ServiceRequest;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use log::info;
use rss_com_lib::{PASSWORD_HEADER, USER_ID_HEADER};

pub const AUTH_COOKIE_NAME: &str = "auth";

#[derive(Default)]
pub struct AuthData {
    users: Users,
}

impl AuthData {
    pub fn new() -> Self {
        Default::default()
    }

    /// TODO (Wybe 2022-07-12): Take encrypted password instead of raw string.
    /// TODO (Wybe 2022-07-12): Have a maximum to the user name length?
    /// TODO (Wybe 2022-07-12): Instead of the username to log-in, use an email address?
    /// TODO (Wybe 2022-07-12): Generate a user id, instead of taking one.
    pub fn new_user(&mut self, user_info: UserInfo) {
        self.users.insert(user_info.id, user_info);
    }

    /// TODO (Wybe 2022-07-11): Implement storing session ids instead of user ids.
    /// TODO (Wybe 2022-07-12): Check whether this user is allowed to access this url.
    pub fn authenticate_user_id(
        &self,
        user_id: Option<String>,
        _request: &ServiceRequest,
    ) -> Option<AuthenticationResult> {
        info!("Authentication attempt: {:?}", user_id);

        user_id
            .and_then(|user_id_string| UserId::from_str(&user_id_string))
            .and_then(|id| self.users.get(&id))
            .map(|info| AuthenticationResult {
                user: info.get_request_info(),
            })
    }

    pub fn validate_password(&self, user_name: &str, password: &str) -> Option<UserId> {
        if let Some((&id, info)) = self.users.iter().find(|(_, info)| info.name == user_name) {
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
#[get("/test_auth_cookie")]
pub async fn test_auth_cookie(auth: Authenticated) -> impl Responder {
    info!(
        "User `{}` connected with valid identity cookie",
        auth.user_name()
    );
    // The fact that the `Authenticated` struct was available means the cookie was valid.
    HttpResponse::Ok().finish()
}

/// Validates user id and password, and sets an identity cookie if they are valid.
#[get("/login")]
pub async fn login(
    req: HttpRequest,
    id: Identity,
    auth_data: web::Data<AuthData>,
) -> impl Responder {
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
            info!("Logging in `{}` with password", user_name);
            // Login valid, set the auth cookie so the user doesn't need to login all the time.
            // TODO (Wybe 2022-07-11): Generate and remember the session id somewhere.
            id.remember(user_id.0.to_string());
            HttpResponse::Ok().finish()
        } else {
            HttpResponse::Unauthorized().finish()
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

/// Logs out the user by forgetting the authentication cookie.
#[get("/logout")]
pub async fn logout(id: Identity, auth: Authenticated) -> impl Responder {
    info!("Logging out `{}`", auth.user_name());

    id.forget();
    HttpResponse::Ok().finish()
}
