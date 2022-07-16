#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod body;

pub const USER_ID_HEADER: &str = "user_id";
pub const PASSWORD_HEADER: &str = "user_pass";

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RssFeedId(u32);
