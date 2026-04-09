use dialoguer::Password;
use dialoguer::theme::ColorfulTheme;
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
        let dirs = ProjectDirs::from("com", "shadowmkj", "leetrs")?;
        Some(dirs.config_dir().join("credentials.json"))
    }

    fn get_data_path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("com", "shadowmkj", "leetrs")?;
        Some(dirs.config_dir().join("data.json"))
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

/// Handles prompting the user to paste their tokens manually
pub fn manual_auth_flow() -> Result<LeetCodeCredentials, String> {
    println!("\nPlease extract your cookies from your browser session.");
    println!("(Developer Tools -> Application -> Cookies -> leetcode.com)\n");

    let session_cookie = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter LEETCODE_SESSION cookie")
        .interact()
        .map_err(|e| e.to_string())?;

    let csrf_token = Password::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter csrftoken cookie")
        .interact()
        .map_err(|e| e.to_string())?;

    Ok(LeetCodeCredentials {
        session_cookie,
        csrf_token,
    })
}

/// Automatically extracts LeetCode cookies from the specified browser
pub fn auto_extract_flow(browser: &str) -> Result<LeetCodeCredentials, String> {
    println!("\n🔍 Attempting to extract cookies from {}...", browser);

    // We only want to query cookies belonging to LeetCode to speed up the process
    let domains = Some(vec!["leetcode.com".to_string()]);

    let cookies = match browser {
        "chrome" => {
            rookie::chrome(domains).map_err(|e| format!("Chrome extraction failed: {}", e))?
        }
        "firefox" => {
            rookie::firefox(domains).map_err(|e| format!("Firefox extraction failed: {}", e))?
        }
        _ => return Err("Unsupported browser".into()),
    };

    let mut session_cookie = None;
    let mut csrf_token = None;

    // Search the returned cookies for the two we care about
    for cookie in cookies {
        if cookie.name == "LEETCODE_SESSION" {
            session_cookie = Some(cookie.value);
        } else if cookie.name == "csrftoken" {
            csrf_token = Some(cookie.value);
        }
    }

    match (session_cookie, csrf_token) {
        (Some(session), Some(csrf)) => Ok(LeetCodeCredentials {
            session_cookie: session,
            csrf_token: csrf,
        }),
        _ => Err(
            "Could not find both LEETCODE_SESSION and csrftoken in the browser's database.".into(),
        ),
    }
}
