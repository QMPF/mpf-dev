mod config;
mod commands;

use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "mpf-dev")]
#[command(about = "MPF Development Environment CLI Tool")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Download and install MPF SDK
    Setup {
        /// SDK version to install (default: latest)
        #[arg(short, long)]
        version: Option<String>,
    },
    
    /// List installed SDK versions
    Versions,
    
    /// Switch to a specific SDK version
    Use {
        /// Version to use
        version: String,
    },
    
    /// Register a component for source development
    Link {
        /// Component name (e.g., http-client, ui-components, plugin-orders, host)
        component: String,
        
        /// Path to built library directory
        #[arg(long)]
        lib: Option<String>,
        
        /// Path to QML modules directory
        #[arg(long)]
        qml: Option<String>,
        
        /// Path to plugin directory (for plugins)
        #[arg(long)]
        plugin: Option<String>,
        
        /// Path to include directory (for headers)
        #[arg(long, alias = "include")]
        headers: Option<String>,
        
        /// Path to executable binary directory (for host component)
        #[arg(long)]
        bin: Option<String>,
        
        /// Path to host build output root (auto-derives bin and qml)
        #[arg(long)]
        host: Option<String>,
    },
    
    /// Unregister a component from source development
    Unlink {
        /// Component name
        component: String,
    },
    
    /// Show current development configuration status
    Status,
    
    /// Print environment variables for manual shell setup
    Env,
    
    /// Run MPF host with development overrides
    Run {
        /// Enable debug mode
        #[arg(short, long)]
        debug: bool,
        
        /// Additional arguments to pass to mpf-host
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Manage full-source workspace (all components from source)
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
}

#[derive(Subcommand)]
enum WorkspaceAction {
    /// Initialize a new workspace with all MPF components
    Init {
        /// Workspace directory (default: current directory)
        #[arg(short, long)]
        path: Option<String>,
    },
    
    /// Build all components in workspace
    Build {
        /// Build type: Debug or Release
        #[arg(short, long, default_value = "Debug")]
        config: String,
    },
    
    /// Run mpf-host from workspace
    Run {
        /// Additional arguments to pass to mpf-host
        #[arg(last = true)]
        args: Vec<String>,
    },
    
    /// Show workspace status
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Setup { version } => commands::setup(version).await,
        Commands::Versions => commands::versions(),
        Commands::Use { version } => commands::use_version(&version),
        Commands::Link { component, lib, qml, plugin, headers, bin, host } => {
            commands::link(&component, lib, qml, plugin, headers, bin, host)
        }
        Commands::Unlink { component } => commands::unlink(&component),
        Commands::Status => commands::status(),
        Commands::Env => commands::env_vars(),
        Commands::Run { debug, args } => commands::run(debug, args),
        Commands::Workspace { action } => match action {
            WorkspaceAction::Init { path } => commands::workspace_init(path),
            WorkspaceAction::Build { config } => commands::workspace_build(&config),
            WorkspaceAction::Run { args } => commands::workspace_run(args),
            WorkspaceAction::Status => commands::workspace_status(),
        },
    }
}
