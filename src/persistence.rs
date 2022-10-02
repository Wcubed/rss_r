use log::{info, warn};
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

/// TODO (Wybe 2022-07-12): Make target directory configurable. And add warning that that directory should only be readable/writable by this program.
const PERSISTENCE_DIR: &str = "persistence";

pub trait SaveInRonFile: Sized + Default + Serialize + DeserializeOwned {
    /// File that the object should be saved to.
    /// The path is interpreted relative to the root of the persistent save directory.
    const FILE_NAME: &'static str;

    /// TODO (Wybe 2022-07-12): Guard against multiple threads writing to the same file at once.
    /// TODO (Wybe 2022-09-24): Can we make saving atomic? So that either we _did_ save the new state, or we didn't, no corrupted .ron files on disk.
    /// TODO (Wybe 2022-07-12): Handle errors.
    /// TODO (Wybe 2022-07-18): Make saving asynchronous, and happen in a background thread? maybe using `actix_web::rt::spawn_blocking();`
    fn save(&self) {
        info!("Saving {}", Self::FILE_NAME);

        let mut path = PathBuf::from(PERSISTENCE_DIR);
        fs::create_dir_all(&path).unwrap_or_else(|_| {
            panic!(
                "Could not create persistence directory: `{}`",
                PERSISTENCE_DIR
            )
        });

        path.push(Self::FILE_NAME);

        match to_string_pretty(self, PrettyConfig::default()) {
            Ok(serialized) => {
                fs::write(&path, serialized)
                    .map_err(|e| warn!("Could not save to `{}`: {}", path.display(), e));
            }
            Err(e) => {
                warn!("Could not convert to RON for `{}`: {}", path.display(), e);
            }
        };
    }

    /// TODO (Wybe 2022-07-12): Handle and log errors.
    fn load() -> Option<Self> {
        let mut path = PathBuf::from(PERSISTENCE_DIR);
        path.push(Self::FILE_NAME);

        if let Ok(contents) = fs::read_to_string(path) {
            let result = ron::from_str(&contents);
            result.ok()
        } else {
            None
        }
    }

    /// Calls [load()](SaveInRonFile::load()) internally.
    fn load_or_default() -> Self {
        Self::load().unwrap_or_default()
    }
}
