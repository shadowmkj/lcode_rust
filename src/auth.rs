use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LeetCodeCredentials {
    pub session_cookie: String,
    pub csrf_token: String,
}

impl LeetCodeCredentials {
    fn get_config_path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("com", "shadow", "lcode")?;
        Some(dirs.config_dir().join("credentials.json"))
    }

    pub fn load() -> Option<Self> {
        let config_path = Self::get_config_path()?;
        let file_contents = std::fs::read_to_string(config_path).ok()?;
        serde_json::from_str(&file_contents).ok()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = Self::get_config_path().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine config directory",
            )
        })?;

        // Ensure the parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        println!("writing to {:?}", &config_path);
        fs::write(config_path, json)?;
        Ok(())
    }
}
