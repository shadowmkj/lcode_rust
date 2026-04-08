use clap::{Parser, Subcommand};
use dialoguer::{Password, Select, theme::ColorfulTheme};
use lcode_rust::auth::LeetCodeCredentials;

#[derive(Parser, Debug)]
#[command(name = "lcode")]
#[command(about = "A Neovim-integrated LeetCode TUI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Authenticate with LeetCode
    Auth,
    /// Launch the TUI (Placeholder for now)
    Tui,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Auth => {
            println!("🔒 LeetCode Authentication\n");

            let options = &[
                "Paste tokens manually",
                "Extract from Firefox",
                "Extract from Chrome",
            ];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("How would you like to authenticate?")
                .default(0)
                .items(&options[..])
                .interact()
                .unwrap();

            let credentials_result = match selection {
                0 => manual_auth_flow(),
                1 => auto_extract_flow("firefox"),
                2 => auto_extract_flow("chrome"),
                _ => unreachable!(),
            };

            match credentials_result {
                Ok(creds) => match creds.save() {
                    Ok(_) => println!(
                        "\n✅ Authentication successful! Credentials saved to ~/.config/lcode/credentials.json"
                    ),
                    Err(e) => eprintln!("\n❌ Failed to save credentials: {}", e),
                },
                Err(e) => {
                    eprintln!("\n❌ Authentication failed: {}", e);
                    if selection != 0 {
                        eprintln!(
                            "Tip: Make sure you are logged into leetcode.com on that browser, or try the manual option."
                        );
                    }
                }
            }
        }
        Commands::Tui => {
            println!("TUI interface coming soon in Phase 2!");
        }
    }
}

/// Handles prompting the user to paste their tokens manually
fn manual_auth_flow() -> Result<LeetCodeCredentials, String> {
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
fn auto_extract_flow(browser: &str) -> Result<LeetCodeCredentials, String> {
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
