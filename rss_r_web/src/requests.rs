use crate::requests::HttpStatus::Other;
use poll_promise::Promise;
use serde::Serialize;
use std::collections::HashMap;

pub struct Requests {
    promises: HashMap<ApiEndpoint, Promise<ehttp::Result<ehttp::Response>>>,
    /// If a promise returns with a `401: Unauthenticated`, this will go false.
    authenticated: bool,
    /// Needed to queue a redraw on the gui upon receiving a response.
    context: egui::Context,
}

impl Requests {
    pub fn new(ctx: egui::Context) -> Self {
        Requests {
            promises: HashMap::new(),
            authenticated: false,
            context: ctx,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn set_authenticated(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
    }

    /// TODO (Wybe 2022-07-16): Add timeout
    pub fn poll(&self) {
        for promise in self.promises.values() {
            promise.ready();
        }
    }

    /// Creates a new, empty request for the given endpoint.
    /// Overwrites any request that currently exists for that endpoint.
    pub fn new_request_without_body(&mut self, endpoint: ApiEndpoint) {
        self.new_request(endpoint, |_| {})
    }

    /// Creates a new request for the given endpoint.
    /// Overwrites any request that currently exists for that endpoint.
    pub fn new_request<F>(&mut self, endpoint: ApiEndpoint, mut request_fn: F)
    where
        F: FnMut(&mut ehttp::Request),
    {
        let mut request = endpoint.request();
        request_fn(&mut request);

        self.add_request(endpoint, request);
    }

    /// Creates a new request for the given endpoint.
    /// Overwrites any request that currently exists for that endpoint.
    pub fn new_request_with_json_body(&mut self, endpoint: ApiEndpoint, body: impl Serialize) {
        let mut request = endpoint.request_with_body(serde_json::to_vec(&body).unwrap());
        //TODO (Wybe 2022-07-16): Make this no longer magic strings, but constants somewhere.
        request
            .headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        self.add_request(endpoint, request);
    }

    fn add_request(&mut self, endpoint: ApiEndpoint, request: ehttp::Request) {
        let (sender, promise) = Promise::new();
        let ctx = self.context.clone();
        ehttp::fetch(request, move |response| {
            // Wake up UI thread.
            ctx.request_repaint();
            sender.send(response)
        });

        self.promises.insert(endpoint, promise);
    }

    /// Checks whether a request has been made.
    /// Does not check whether the request is ready or not.
    pub fn has_request(&self, endpoint: ApiEndpoint) -> bool {
        self.promises.contains_key(&endpoint)
    }

    /// TODO (Wybe 2022-07-11): Make this use proper response types instead of strings.
    /// Returns `Some` if a request returned successfully, and clears the request.
    pub fn ready(&mut self, endpoint: ApiEndpoint) -> Option<Response> {
        let mut promise_handled = false;

        let result = self.promises.get(&endpoint).and_then(|promise| {
            let ready = promise.ready();
            if let Some(result) = ready {
                let return_value = match result {
                    Ok(response) => {
                        let status_code = HttpStatus::from_u16(response.status);

                        match status_code {
                            HttpStatus::Ok => {
                                Response::Ok(response.text().unwrap_or("").to_string())
                            }
                            HttpStatus::Unauthorized => {
                                self.authenticated = false;
                                Response::NotOk(status_code)
                            }
                            _ => Response::NotOk(status_code),
                        }
                    }
                    // TODO (Wybe 2022-07-11): Handle errors. For example: the server isn't online.
                    Err(_e) => Response::Error,
                };
                // We are done with this promise, so it can be cleaned up.
                promise_handled = true;
                Some(return_value)
            } else {
                None
            }
        });

        if promise_handled {
            self.promises.remove(&endpoint);
        }

        result
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ApiEndpoint {
    TestAuthCookie,
    Login,
    Logout,
    IsUrlAnRssFeed,
    AddFeed,
    ListFeeds,
    /// Get all the entries in the requested feeds.
    GetFeedEntries,
}

impl ApiEndpoint {
    // TODO (Wybe 2022-07-14): Add some way for requests to be idempotent. So that if the same request
    //                         is sent again, it is detected as duplicate, and nothing happens.
    pub fn request(&self) -> ehttp::Request {
        self.request_with_body(Vec::new())
    }

    pub fn request_with_body(&self, body: Vec<u8>) -> ehttp::Request {
        let endpoint = match self {
            Self::TestAuthCookie => "test_auth_cookie",
            Self::Login => "login",
            Self::Logout => "logout",
            Self::IsUrlAnRssFeed => "is_url_an_rss_feed",
            Self::AddFeed => "add_feed",
            Self::ListFeeds => "list_feeds",
            Self::GetFeedEntries => "get_feed_entries",
        };

        ehttp::Request::post(format!("../api/{}", endpoint), body)
    }
}

#[derive(Debug)]
pub enum Response {
    Ok(String),
    NotOk(HttpStatus),
    Error,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum HttpStatus {
    Ok,
    Unauthorized,
    Other(u16),
}

impl HttpStatus {
    pub fn from_u16(value: u16) -> Self {
        match value {
            200 => Self::Ok,
            401 => Self::Unauthorized,
            _ => Other(value),
        }
    }
}
