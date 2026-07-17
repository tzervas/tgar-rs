//! `tgar` — CLI entry for the Rust TG Agent Relay port.

use std::io::{self, Read};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use tgar_core::{default_config_path, load_config, VERSION};
use tgar_telegram::{
    format_page_payloads, paginate, SendMessageParams, TelegramBot, UreqHttpClient,
};

#[derive(Parser)]
#[command(name = "tgar", version = VERSION, about = "TG Agent Relay (Rust)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Print version string.
    Version,
    /// Config surface (Phase 0 stub).
    Config {
        #[command(subcommand)]
        command: ConfigCmd,
    },
    /// Paginate and send (or dry-run) outbound text.
    Send {
        /// Message body (if omitted, read stdin).
        #[arg(long)]
        text: Option<String>,
        /// Pagination size in characters (default 3500).
        #[arg(long, default_value_t = 3500)]
        page_size: usize,
        /// Destination chat id (live send). Falls back to `ALLOWED_CHAT_ID` env.
        #[arg(long)]
        chat_id: Option<String>,
        /// Force dry-run: print pages only, no HTTP.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Validate `relay.toml` exists and parses (schema parity later).
    Check {
        /// Path to relay.toml (default: `RELAY_CONFIG` or `./relay.toml`).
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("tgar: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Version) => {
            println!("{VERSION}");
            Ok(())
        }
        Some(Commands::Config {
            command: ConfigCmd::Check { path },
        }) => cmd_config_check(path),
        Some(Commands::Send {
            text,
            page_size,
            chat_id,
            dry_run,
        }) => cmd_send(text, page_size, chat_id, dry_run),
    }
}

fn cmd_config_check(path: Option<PathBuf>) -> Result<(), String> {
    let path = path.unwrap_or_else(default_config_path);
    let cfg = load_config(&path)?;
    let page = cfg
        .page_size
        .map(|n| n.to_string())
        .unwrap_or_else(|| "default".into());
    println!(
        "ok: {} (tables=[{}]; page_size={}; stub — full schema Phase 0+)",
        cfg.path.display(),
        cfg.tables.join(", "),
        page
    );
    Ok(())
}

fn cmd_send(
    text: Option<String>,
    page_size: usize,
    chat_id: Option<String>,
    dry_run: bool,
) -> Result<(), String> {
    let msg = match text {
        Some(t) => t,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            buf
        }
    };

    if msg.is_empty() {
        return Ok(());
    }

    let pages = paginate(&msg, page_size);
    let payloads = format_page_payloads(&pages);

    let token = std::env::var("BOT_TOKEN").unwrap_or_default();
    let chat = chat_id
        .or_else(|| std::env::var("ALLOWED_CHAT_ID").ok())
        .unwrap_or_default();

    let do_dry_run = dry_run || token.is_empty();

    if do_dry_run {
        if token.is_empty() && !dry_run {
            eprintln!("tgar: BOT_TOKEN unset — dry-run (printing pages only)");
        }
        for (i, page) in payloads.iter().enumerate() {
            if payloads.len() > 1 {
                eprintln!("--- page {}/{} ---", i + 1, payloads.len());
            }
            print!("{page}");
            if !page.ends_with('\n') {
                println!();
            }
        }
        return Ok(());
    }

    if chat.is_empty() {
        return Err(
            "live send requires --chat-id or ALLOWED_CHAT_ID when BOT_TOKEN is set".into(),
        );
    }

    let bot = TelegramBot::new(token, UreqHttpClient);
    for page in &payloads {
        bot.send_message(SendMessageParams {
            chat_id: &chat,
            text: page,
            parse_mode: None,
            message_thread_id: None,
            reply_markup: None,
        })
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tgar_telegram::{format_page_payloads, paginate};

    #[test]
    fn dev_version_string() {
        assert_eq!(tgar_core::VERSION, "0.1.0-dev");
    }

    #[test]
    fn send_pagination_smoke() {
        let pages = paginate("hello", 100);
        let payloads = format_page_payloads(&pages);
        assert_eq!(payloads, vec!["hello".to_string()]);
    }
}
