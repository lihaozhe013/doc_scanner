use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::i18n::LanguagePreference;

/// Persistent user preferences stored outside the repository and session
/// files. Only the language setting exists today; new settings should be
/// optional (`#[serde(default)]`) so older files keep loading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub language: LanguagePreference,
}

pub fn load() -> Preferences {
    let path = match preferences_path() {
        Some(path) => path,
        None => {
            warn!("[preferences] no writable configuration directory found");
            return Preferences::default();
        }
    };
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str::<Preferences>(&contents)
            .inspect(|preferences| {
                info!(
                    "[preferences] loaded language {:?}",
                    preferences.language
                );
            })
            .map_err(|error| {
                warn!(
                    "[preferences] ignoring malformed preferences file: {error}"
                );
            })
            .unwrap_or_default(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Preferences::default()
        }
        Err(error) => {
            warn!("[preferences] could not read preferences file: {error}");
            Preferences::default()
        }
    }
}

pub fn save(preferences: &Preferences) -> Result<(), String> {
    let path = preferences_path()
        .ok_or_else(|| "no configuration directory is available".to_owned())?;
    let contents = serde_json::to_string_pretty(preferences)
        .map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    fs::write(&path, contents)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    info!("[preferences] saved language {:?}", preferences.language);
    Ok(())
}

fn preferences_path() -> Option<PathBuf> {
    let base = dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."));
    Some(base.join("document-scanner").join("preferences.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_through_json() {
        let preferences = Preferences {
            language: LanguagePreference::Chinese,
        };
        let json = serde_json::to_string(&preferences).unwrap();
        let restored: Preferences = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.language, preferences.language);
    }

    #[test]
    fn missing_and_unknown_fields_fall_back_to_defaults() {
        let restored: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(restored.language, LanguagePreference::Auto);
        let restored: Preferences =
            serde_json::from_str(r#"{"language":"en-US","future":1}"#).unwrap();
        assert_eq!(restored.language, LanguagePreference::English);
    }
}
