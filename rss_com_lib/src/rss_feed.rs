use crate::Url;
use chrono::{DateTime, TimeZone, Utc};
use feed_rs::model;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;
use std::collections::{hash_map, HashMap, HashSet};
use std::fmt::{Debug, Formatter, Write};

#[derive(Serialize, Deserialize, Debug, Clone, Default, Eq, PartialEq)]
#[serde(default)]
pub struct FeedInfo {
    pub name: String,
    pub tags: HashSet<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct FeedEntries(HashMap<EntryKey, FeedEntry>);

impl FeedEntries {
    pub fn new(entries: HashMap<EntryKey, FeedEntry>) -> Self {
        FeedEntries(entries)
    }

    pub fn inner(self) -> HashMap<EntryKey, FeedEntry> {
        self.0
    }
}

impl std::ops::Deref for FeedEntries {
    type Target = HashMap<EntryKey, FeedEntry>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FeedEntries {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for FeedEntries {
    type Item = (EntryKey, FeedEntry);
    type IntoIter = hash_map::IntoIter<EntryKey, FeedEntry>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Key identifying a [FeedEntry] in a feed.
/// The value is generated using the [blake3::Hasher].
///
/// TODO (Wybe 2022-09-24): Serialize and deserialize as base64? https://github.com/serde-rs/serde/issues/661
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct EntryKey([u8; 32]);

impl EntryKey {
    /// The key is based on the title and date of the entry.
    pub fn from_entry(entry: &FeedEntry) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(entry.title.as_bytes());
        hasher.update(&entry.pub_date.timestamp().to_be_bytes());

        EntryKey(hasher.finalize().into())
    }
}

impl Debug for EntryKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("EntryKey(")?;
        f.write_str(&base64::encode(self.0))?;
        f.write_str(")")
    }
}

impl Serialize for EntryKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // To send hashmaps as json (which is used by actix to send a response body)
        // the keys need to be strings.
        // Also, this is more compact than printing it as a list of base 10 numbers.
        serializer.serialize_str(&base64::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for EntryKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        String::deserialize(deserializer)
            .and_then(|string| {
                base64::decode(&string).map_err(|err| Error::custom(err.to_string()))
            })
            .and_then(|byte_vec| {
                byte_vec.try_into().map_err(|_| {
                    Error::custom("Expected 32 bytes as feed entry key, but didn't get 32")
                })
            })
            .map(EntryKey)
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug, Default)]
#[serde(default)]
pub struct FeedEntry {
    pub title: String,
    /// Link to the original content.
    pub link: Option<Url>,
    /// If an rss feed includes an entry with no date, it will get a default date in the past.
    pub pub_date: DateTime<Utc>,
    pub read: bool,
}

impl FeedEntry {
    pub fn from_raw_feed_entry(item: &model::Entry) -> (EntryKey, Self) {
        // If the entry has no publication date, we will us a default date far in the past.
        let default_date = Utc.ymd(1900, 1, 1).and_hms(1, 1, 1);

        let pub_date = item
            .published
            .as_ref()
            .cloned()
            .unwrap_or_else(|| default_date.into())
            .with_timezone(&Utc);

        let entry = Self {
            title: match &item.title {
                Some(title) => title.content.clone(),
                None => "No title".to_string(),
            },
            link: item
                .links
                .first()
                .clone()
                .map(|link| Url::new(link.href.clone())),
            pub_date,
            read: false,
        };
        let key = EntryKey::from_entry(&entry);
        (key, entry)
    }
}

impl PartialOrd for FeedEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FeedEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Normally one would compare self to other.
        // However, the FeedEntries should be sorted with newest first,
        // so we compare the other with self.
        let mut ord = other.pub_date.cmp(&self.pub_date);

        if ord == Ordering::Equal {
            ord = self.title.cmp(&other.title);
        }

        if ord == Ordering::Equal {
            ord = self.link.cmp(&other.link);
        }

        ord
    }
}

#[cfg(test)]
mod tests {
    use crate::rss_feed::{EntryKey, FeedEntry};
    use chrono::{TimeZone, Utc};
    use pretty_assertions::assert_eq;
    use serde_json;

    /// If the hashing algorithm used to generate [FeedEntry] keys changes, then the keys in the saved
    /// persistence files won't match anymore with the ones generated by the application.
    ///
    /// This test is here to alert us that this has happened, and additional checks should be put
    /// in place to migrate the data.
    #[test]
    fn hash_algorithm_change_guard() {
        // Given
        let entry = FeedEntry {
            title: "Title".to_owned(),
            link: None,
            pub_date: Utc.ymd(2022, 9, 10).and_hms(1, 3, 4),
            read: false,
        };

        // When
        let key = EntryKey::from_entry(&entry);

        // Then
        assert_eq!(
            key,
            EntryKey([
                175, 242, 19, 86, 102, 187, 89, 25, 187, 21, 254, 107, 18, 185, 250, 130, 34, 30,
                74, 67, 249, 139, 4, 71, 134, 86, 111, 146, 201, 10, 235, 81
            ])
        );
    }

    #[test]
    fn test_entry_key_serialization() {
        // Given
        let key = EntryKey([3; 32]);

        // When
        let string = serde_json::to_string(&key).unwrap();

        // Then
        assert_eq!(string, "\"AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=\"");
    }

    #[test]
    fn test_entry_key_deserialization() {
        // When
        let key: EntryKey =
            serde_json::from_str("\"AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=\"").unwrap();

        // Then
        assert_eq!(key, EntryKey([3; 32]));
    }
}
