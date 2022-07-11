//! For more info on how middleware works, and what each component is doing, see
//! https://imfeld.dev/writing/actix-web-middleware
//! A large part of this file's code also comes from there.

use crate::error::Error;
use crate::users::{UserId, UserInfo};
use actix_identity::RequestIdentity;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{FromRequest, HttpMessage};
use actix_web_lab::__reexports::futures_util::future::LocalBoxFuture;
use actix_web_lab::__reexports::futures_util::FutureExt;
use std::future::{ready, Ready};
use std::rc::Rc;

pub const AUTH_COOKIE_NAME: &str = "auth";

#[derive(Clone)]
pub struct AuthData;

impl AuthData {
    /// TODO (Wybe 2022-07-11): Implement
    async fn authenticate(
        &self,
        session_id: Option<String>,
        _request: &ServiceRequest,
    ) -> Option<AuthenticationResult> {
        session_id
            .and_then(|session_id_string| SessionId::from_str(&session_id_string))
            .map(|id| AuthenticationResult {
                session_id: id,
                // TODO (Wybe 2022-07-11): Map session to user.
                user: UserInfo {
                    id: UserId(0),
                    name: "".to_string(),
                },
            })
    }
}

/// TODO (Wybe 2022-07-11): Add authentication info.
pub struct AuthenticationResult {
    session_id: SessionId,
    user: UserInfo,
}

impl AuthenticationResult {
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub fn user_id(&self) -> &UserId {
        &self.user.id
    }

    pub fn user_name(&self) -> &str {
        &self.user.name
    }
}

pub struct SessionId(u32);

impl SessionId {
    fn from_str(string: &str) -> Option<Self> {
        string.parse::<u32>().ok().map(|id| SessionId(id))
    }
}

pub type AuthenticationInfo = Rc<AuthenticationResult>;

/// Parameter for services that indicates a user is authenticated.
pub struct Authenticated(AuthenticationInfo);

impl FromRequest for Authenticated {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let value = req.extensions().get::<AuthenticationInfo>().cloned();
        let result = match value {
            Some(v) => Ok(Authenticated(v)),
            None => Err(Error::AuthenticationError),
        };
        ready(result)
    }
}

impl std::ops::Deref for Authenticated {
    type Target = AuthenticationInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct MaybeAuthenticated(Option<AuthenticationInfo>);

impl MaybeAuthenticated {
    pub fn is_some(&self) -> bool {
        self.0.is_some()
    }

    pub fn inner(&self) -> &Option<AuthenticationInfo> {
        &self.0
    }
}

impl FromRequest for MaybeAuthenticated {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(
        req: &actix_web::HttpRequest,
        _payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        let value = req.extensions().get::<AuthenticationInfo>().cloned();
        ready(Ok(MaybeAuthenticated(value)))
    }
}

/// Checks whether a user is logged in using an identity cookie.
/// Adds the [AuthenticationInfo] to the request if the user is logged in.
/// Services can require the user to be logged in by requiring [Authenticated] as parameter.
/// If the user is not _required_ to be logged in, one can use [MaybeAuthenticated] instead.
pub struct AuthenticateMiddleware<S> {
    auth_data: Rc<AuthData>,
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Clone the Rc pointers so we can move them into the async block.
        let srv = self.service.clone();
        let auth_data = self.auth_data.clone();

        async move {
            // Get the session cookie value, if it exists.
            let id = req.get_identity();
            // See if we can match it to a user.
            let auth = auth_data.authenticate(id, &req).await;
            if let Some(auth) = auth {
                // If we found a user, add it to the request extensions
                // for later retrieval.
                req.extensions_mut()
                    .insert::<AuthenticationInfo>(Rc::new(auth));
            }

            let res = srv.call(req).await?;

            Ok(res)
        }
        .boxed_local()
    }
}

pub struct AuthenticateMiddlewareFactory {
    auth_data: Rc<AuthData>,
}

impl AuthenticateMiddlewareFactory {
    pub fn new(auth_data: AuthData) -> Self {
        AuthenticateMiddlewareFactory {
            auth_data: Rc::new(auth_data),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuthenticateMiddlewareFactory
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = AuthenticateMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticateMiddleware {
            auth_data: self.auth_data.clone(),
            service: Rc::new(service),
        }))
    }
}
