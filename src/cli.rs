use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "lfr-local",
    version = "0.1.0",
    about = "Liferay Local Instance Manager"
)]
pub struct App {
    #[command(subcommand)]
    pub command: AppCommands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum AppCommands {
    /// Initialize a new Liferay bundle from a remote source
    Init {
        /// Liferay product ID (e.g., dxp-2024.q1.0, portal-7.4.3.112)
        #[arg(short, long)]
        product: Option<String>,
        /// Direct URL to a bundle zip file
        #[arg(short, long)]
        url: Option<String>,
        /// Base URL for product resolution (overrides defaults)
        #[arg(long)]
        base_url: Option<String>,
        /// Target directory name for the bundle
        #[arg(short, long)]
        name: String,
    },
    /// Configure a Liferay bundle for a specific instance ID
    Configure {
        /// Unique ID (e.g., 1, 2) to derive ports (8180, 8280) and sessions
        instance_id: u16,
        /// Path to the Liferay Workspace
        #[arg(short, long)]
        workspace_path: Option<PathBuf>,
        /// Optional custom HSQL database name
        #[arg(short, long)]
        db_name: Option<String>,
        /// Wipe persistent data (Elasticsearch indexes and HSQL)
        #[arg(long)]
        clear_data: bool,
    },
    /// Display a summary of the current Liferay configuration
    Summary,
    /// Check which Liferay instances are currently running
    Status {
        /// Optional: Check a specific instance ID
        instance_id: Option<u16>,
    },
    /// Kill a running Liferay instance by its ID
    Kill {
        /// The instance ID to terminate
        instance_id: u16,
    },
    /// Reset the Liferay environment to a clean state
    Reset {
        /// Path to the Liferay Workspace
        #[arg(short, long)]
        workspace_path: Option<PathBuf>,
        /// Wipe all persistent data (Databases & Indexes)
        #[arg(long)]
        all: bool,
        /// Reset portal-ext.properties (Session Cookie and DB URL)
        #[arg(long)]
        props: bool,
        /// Reset server.xml ports to defaults (8080, 8005, etc.)
        #[arg(long)]
        ports: bool,
    },
}
