//! For more info on how middleware works, and what each component is doing, see
//! https://imfeld.dev/writing/actix-web-middleware
//! A large part of this file's code also comes from there.

use crate::auth::{AuthData, AuthenticationResult};
use crate::error::Error;
use actix_identity::RequestIdentity;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, FromRequest, HttpMessage};
use actix_web_lab::__reexports::futures_util::future::LocalBoxFuture;
use actix_web_lab::__reexports::futures_util::FutureExt;
use log::error;
use std::future::{ready, Ready};
use std::rc::Rc;

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

/// Checks whether a user is logged in using an identity cookie.
/// Adds the [AuthenticationInfo] to the request if the user is logged in.
/// Services can require the user to be logged in by requiring [Authenticated] as parameter.
///
/// Relies on [AuthData] to be in the web apps data.
pub struct AuthenticateMiddleware<S> {
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
        if let Some(auth_data) = req.app_data::<web::Data<AuthData>>() {
            // Get the session cookie value, if it exists.
            let id = req.get_identity();
            // See if we can match it to a user.
            let auth = auth_data.authenticate_user_id(id, &req);
            if let Some(auth) = auth {
                // If we found a user, add it to the request extensions
                // for later retrieval.
                req.extensions_mut()
                    .insert::<AuthenticationInfo>(Rc::new(auth));
            }

            async move {
                let res = srv.call(req).await?;

                Ok(res)
            }
            .boxed_local()
        } else {
            error!("AuthData is not available in web data. Cannot authenticate users.");
            std::process::exit(1);
        }
    }
}

pub struct AuthenticateMiddlewareFactory;

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
            service: Rc::new(service),
        }))
    }
}
