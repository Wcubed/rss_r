use crate::{cookie, SaveInRonFile};
use serde::{Deserialize, Serialize};

/// If a value is not found in the saved config, serde will use the default value.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct ApplicationConfig {
    /// Hostname that the server will be located at.
    /// Used for when absolute urls need to be generated.
    pub hostname: String,
    /// Port the server will listen to.
    pub port: u32,
    /// When the application is behind a proxy, requests might have a prefix.
    /// Fill in the prefix here to deal with it properly.
    ///
    /// For example, if the proxy forwards requests from `/feedreader`, then
    /// a request for `/app/index.html` will arrive at this server as `/feedreader/app/index.hml`.
    /// A route_prefix of `/feedreader` will make sure all the routes still work.
    pub route_prefix: String,

    /// The master key for creating session cookies.
    pub session_key: Vec<u8>,
}

impl ApplicationConfig {
    pub fn binding_ip(&self) -> String {
        format!("0.0.0.0:{}", self.port)
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            hostname: "localhost".to_string(),
            port: 8443,
            route_prefix: "".to_string(),
            // If no key is supplied, generate one.
            session_key: cookie::Key::generate().master().to_vec(),
        }
    }
}

impl SaveInRonFile for ApplicationConfig {
    const FILE_NAME: &'static str = "app_config.ron";
}
