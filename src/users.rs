pub struct UserInfo {
    pub id: UserId,
    pub name: String,
}

// TODO (Wybe 2022-07-11): Make internal id private?
pub struct UserId(pub u32);
