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
use dialoguer::{Password, Select, theme::ColorfulTheme};
use leetrs::{auth::LeetCodeCredentials, client::LeetCodeClient, models::Language, picker::Picker};

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
async fn main() {
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
            println!("TUI interface coming soon!");
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
                    return;
                }
            };
            let client = match LeetCodeClient::new(creds) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to initialize client: {}", e);
                    return;
                }
            };
            let picker = Picker::new(client);
            if let Ok((code, desc)) = picker.pick(identifier, language).await {
                // 4. launch neovim with a vertical split
                println!("🚀 launching neovim...");
                if !*preview {
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
        Commands::Submit { file } => {
            let creds = match LeetCodeCredentials::load() {
                Some(c) => c,
                None => {
                    eprintln!("❌ Not authenticated. Please run `leetrs auth` first.");
                    return;
                }
            };

            let client = match LeetCodeClient::new(creds) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to initialize client: {}", e);
                    return;
                }
            };

            // 1. Read the file content
            let code = match std::fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("❌ Failed to read file '{}': {}", file, e);
                    return;
                }
            };

            // 2. Extract the slug from the filename (e.g., "two_sum.rs" -> "two-sum")
            let path = std::path::Path::new(&file);
            let file_stem = path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default();
            let slug = file_stem.replace("_", "-");
            println!("🔍 Resolving ID for '{}'...", slug);
            let language = Language::from_extension(
                path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default(),
            );
            // 3. Fetch the question to get its internal ID
            let question = match client.get_question_by_slug(&slug, &language).await {
                Ok(q) => q,
                Err(e) => {
                    eprintln!(
                        "❌ Failed to fetch question ID. Does the filename match the problem slug? Error: {}",
                        e
                    );
                    return;
                }
            };

            // 4. Submit the code
            println!("🚀 Submitting {}...", file);
            let submission_id = match client
                .submit_code(&slug, &question.question_id, language.to_lang_slug(), &code)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    eprintln!("❌ Submission failed: {}", e);
                    return;
                }
            };

            // 5. Poll for results
            println!("⏳ Code queued. Waiting for execution results...");
            let result = match client.check_submission(submission_id).await {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("❌ Failed to check submission status: {}", e);
                    return;
                }
            };

            // 6. Display the formatted results
            println!("\n==================================================");

            let status = result.status_msg.unwrap_or_else(|| "Unknown".to_string());

            if status == "Accepted" {
                // Print Accepted in Green
                // println!("  ✅ \x1b[32m{}\x1b[0m", status);
                println!("  ✅ {}", status);
            } else {
                // Print Errors/Wrong Answers in Red
                // println!("  ❌ \x1b[31m{}\x1b[0m", status);
                println!("  ❌ {}", status);
            }

            println!("==================================================\n");

            if let (Some(correct), Some(total)) = (result.total_correct, result.total_testcases) {
                println!("🧪 Testcases: {} / {} passed", correct, total);
            }

            if status == "Accepted" {
                if let Some(runtime) = result.status_runtime {
                    println!("⏱️  Runtime: {}", runtime);
                }
                if let Some(memory) = result.status_memory {
                    println!("💾 Memory: {}", memory);
                }
            } else if status == "Compile Error" {
                if let Some(err_msg) = result.compile_error {
                    println!("💥 Compiler Output:\n{}", err_msg);
                }
            }
        }
        Commands::Version => {
            println!("leetrs 1.0");
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
