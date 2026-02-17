mod core;
mod utils;
mod cli;

use clap::Parser;
use crate::core::{Workspace, LiferayWorkspace};
use crate::cli::{App, AppCommands};

fn main() {
    let args = App::parse();
    
    // Initialize the workspace abstraction
    let ws = LiferayWorkspace {
        current_dir: std::env::current_dir().unwrap_or_default(),
    };

    match args.command {
        AppCommands::Env { action } => {
            if let Ok(root) = ws.find_root() {
                println!("Action '{}' targeted at root: {:?}", action, root);
            }
        },
        AppCommands::Data { operation } => {
            println!("Executing data operation: {}", operation);
            // This is where you'd call utils::archive or utils::process
        },
    }
}