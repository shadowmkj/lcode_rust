use std::{
    fs::{self},
    io,
    process::Command,
};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{
    Shell,
    aot::{Bash, Fish, Zsh},
    generate,
};
use dialoguer::{Select, theme::ColorfulTheme};
use leetrs::{
    auth::{LeetCodeCredentials, auto_extract_flow, manual_auth_flow},
    client::LeetCodeClient,
    models::Language,
    picker::Picker,
};

#[derive(Parser, Debug)]
#[command(name = "leetrs")]
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
    /// Check auth status
    Status,
    /// Pick a problem
    Pick {
        identifier: String,
        language: Option<Language>,
        #[arg(short, long)]
        preview: bool,
    },
    /// Submit a problem to leetcode
    Submit {
        /// The path to your solution file (e.g., 'two_sum.rs')
        file: String,
    },
    /// Setup autocomplete for shell
    Completion { shell: Shell },
    /// Check leetrs version
    Version,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
                    Ok(_) => println!("\n✅ Authentication successful!"),
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
            let creds =
                leetrs::auth::LeetCodeCredentials::load().expect("Please run `leetrs auth` first.");
            let client = leetrs::client::LeetCodeClient::new(creds).expect("Failed to init client");

            println!("⏳ Fetching problem list... (This takes a few seconds)");

            // 2. Fetch Data
            let problems = client
                .get_problem_list()
                .await
                .expect("Failed to fetch problem list");
            let selected_slug = match leetrs::tui::run_tui(problems).await {
                Ok(slug) => slug,
                Err(e) => {
                    eprintln!("Fatal error in TUI: {e}");
                    return Err(e.into());
                }
            };

            if let Some(slug) = selected_slug {
                let picker = Picker::new(client);
                pick_and_open(picker, &slug, &Some(Language::Python), false).await;
            }
        }
        Commands::Status => {
            match LeetCodeCredentials::load() {
                Some(creds) => {
                    println!("✅ Currently authenticated!");

                    println!("🔑 csrftoken:");
                    println!("{}\n", creds.csrf_token);

                    println!("🔑 LEETCODE_SESSION:");
                    // LEETCODE_SESSION is massive. Printing it nicely so it wraps well.
                    println!("{}", creds.session_cookie);
                }
                None => {
                    eprintln!("❌ Not authenticated. No valid credentials found.");
                    eprintln!("Run `leetrs auth` to set up your account.");
                }
            }
        }
        Commands::Pick {
            identifier,
            language,
            preview,
        } => {
            let creds = match LeetCodeCredentials::load() {
                Some(c) => c,
                None => {
                    eprintln!("❌ Not authenticated. Please run `leetrs auth` first.");
                    return Ok(());
                }
            };
            let client = match LeetCodeClient::new(creds) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to initialize client: {}", e);
                    return Err(e.into());
                }
            };
            let picker = Picker::new(client);
            pick_and_open(picker, identifier, language, *preview).await;
        }
        Commands::Submit { file } => {
            let creds = match LeetCodeCredentials::load() {
                Some(c) => c,
                None => {
                    eprintln!("❌ Not authenticated. Please run `leetrs auth` first.");
                    return Ok(());
                }
            };

            let client = match LeetCodeClient::new(creds) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to initialize client: {}", e);
                    return Err(e.into());
                }
            };

            let picker = Picker::new(client);
            picker.submit(file).await;
        }
        Commands::Version => {
            println!("leetrs 1.0.2 (beta)");
        }
        Commands::Completion { shell } => {
            let mut cmd = Cli::command();

            match shell {
                Shell::Bash => generate(Bash, &mut cmd, "leetrs", &mut io::stdout()),
                Shell::Zsh => generate(Zsh, &mut cmd, "leetrs", &mut io::stdout()),
                Shell::Fish => generate(Fish, &mut cmd, "leetrs", &mut io::stdout()),
                Shell::Elvish => todo!(),
                Shell::PowerShell => todo!(),
                _ => todo!(),
            }
        }
    };

    Ok(())
}

async fn pick_and_open(
    picker: Picker,
    identifier: &String,
    language: &Option<Language>,
    preview: bool,
) {
    if let Ok((code, desc)) = picker.pick(identifier, language).await {
        // 4. launch neovim with a vertical split
        println!("🚀 launching neovim...");
        if !preview {
            let status = Command::new("nvim")
                .arg(&desc)
                .arg("-c")
                .arg(format!("vsplit {}", code)) // Force a vertical split with the code file
                .status();

            match status {
                Ok(exit_status) if exit_status.success() => {
                    println!("\n👋 neovim closed.");
                }
                Ok(exit_status) => {
                    eprintln!("⚠️ neovim exited with an error code: {}", exit_status);
                }
                Err(e) => {
                    eprintln!(
                        "❌ failed to launch neovim. is it installed and in your path? error: {}",
                        e
                    );
                }
            }
        } else {
            let content = fs::read_to_string(desc);
            if let Ok(content) = content {
                print!("{}", content);
            }
        }
    }
}
