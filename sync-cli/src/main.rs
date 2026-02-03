//! # sync-cli
//!
//! CLI tool for testing 0k-Sync protocol.
//!
//! ## Commands
//!
//! - `init`: Initialize device identity
//! - `pair`: Create or join a sync group
//! - `push`: Push data to the sync group
//! - `pull`: Pull data from the sync group
//! - `status`: Show sync status
//!
//! ## Example
//!
//! ```bash
//! # Initialize device
//! sync-cli init --name "My Device"
//!
//! # Create sync group and get invite code
//! sync-cli pair --create
//!
//! # On another device, join the group
//! sync-cli pair --join XXXX-XXXX-XXXX-XXXX
//!
//! # Push data
//! sync-cli push "Hello, sync world!"
//!
//! # Pull data
//! sync-cli pull
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;
mod config;

use commands::{init, pair, pull, push, serve, status};

/// CLI tool for testing 0k-Sync protocol.
#[derive(Parser, Debug)]
#[command(name = "sync-cli")]
#[command(version, about, long_about = None)]
struct Cli {
    /// Data directory for storing device identity and sync state
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Use mock transport instead of real iroh P2P (for testing/demo)
    #[arg(long, global = true)]
    mock: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize device identity
    Init {
        /// Device name
        #[arg(long, short)]
        name: String,
    },

    /// Create or join a sync group
    Pair {
        /// Create a new sync group and display invite code
        #[arg(long, conflicts_with = "join")]
        create: bool,

        /// Join an existing sync group with invite code
        #[arg(long, conflicts_with = "create")]
        join: Option<String>,

        /// Passphrase for the sync group (will prompt if not provided)
        #[arg(long, short)]
        passphrase: Option<String>,
    },

    /// Push data to the sync group
    Push {
        /// Message to push (or use --file)
        message: Option<String>,

        /// File to push
        #[arg(long, short, conflicts_with = "message")]
        file: Option<PathBuf>,
    },

    /// Pull data from the sync group
    Pull {
        /// Only pull data after this cursor
        #[arg(long)]
        after_cursor: Option<u64>,

        /// Maximum number of items to pull
        #[arg(long, default_value = "100")]
        limit: u32,
    },

    /// Show sync status
    Status,

    /// Start a sync server (accepts connections from other devices)
    Serve {
        /// Passphrase for the sync group
        #[arg(long, short)]
        passphrase: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Determine data directory
    let data_dir = match cli.data_dir {
        Some(dir) => dir,
        None => default_data_dir()?,
    };

    // Ensure data directory exists
    tokio::fs::create_dir_all(&data_dir)
        .await
        .context("Failed to create data directory")?;

    match cli.command {
        Commands::Init { name } => {
            init::run(&data_dir, &name).await?;
        }
        Commands::Pair {
            create,
            join,
            passphrase,
        } => {
            if create {
                pair::create(&data_dir, passphrase.as_deref()).await?;
            } else if let Some(code) = join {
                pair::join(&data_dir, &code, passphrase.as_deref()).await?;
            } else {
                anyhow::bail!("Must specify either --create or --join");
            }
        }
        Commands::Push { message, file } => {
            let data = if let Some(msg) = message {
                msg.into_bytes()
            } else if let Some(path) = file {
                tokio::fs::read(&path)
                    .await
                    .context("Failed to read file")?
            } else {
                anyhow::bail!("Must specify message or --file");
            };
            push::run(&data_dir, &data, cli.mock).await?;
        }
        Commands::Pull {
            after_cursor,
            limit: _,
        } => {
            pull::run(&data_dir, after_cursor, cli.mock).await?;
        }
        Commands::Status => {
            status::run(&data_dir).await?;
        }
        Commands::Serve { passphrase } => {
            serve::run(&data_dir, passphrase.as_deref()).await?;
        }
    }

    Ok(())
}

/// Get the default data directory for sync-cli.
fn default_data_dir() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("io", "zerok", "sync-cli")
        .context("Could not determine home directory")?;
    Ok(dirs.data_dir().to_path_buf())
}
