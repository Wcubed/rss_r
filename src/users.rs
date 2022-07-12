use std::collections::HashMap;

/// TODO (Wybe 2022-07-11): Store / load user info on disk.
#[derive(Default)]
pub struct Users(HashMap<UserId, UserInfo>);

impl std::ops::Deref for Users {
    type Target = HashMap<UserId, UserInfo>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Users {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct UserInfo {
    pub id: UserId,
    pub name: String,
    // TODO (Wybe 2022-07-11): Encrypt password according to current best practices.
    //                         Maybe use Argon2, like in https://github.com/dimfeld/ergo/blob/deca6447c4cebdad4e4fa28317a8fcd9f8ed63f2/auth/password.rs
    pub password: String,
}

impl UserInfo {
    pub fn get_request_info(&self) -> UserRequestInfo {
        UserRequestInfo {
            id: self.id,
            name: self.name.clone(),
        }
    }
}

/// User info that is passed to service functions.
pub struct UserRequestInfo {
    pub id: UserId,
    pub name: String,
}

// TODO (Wybe 2022-07-11): Make internal id private?
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct UserId(pub u32);

impl UserId {
    pub fn from_str(string: &str) -> Option<Self> {
        string.parse::<u32>().ok().map(Self)
    }
}
