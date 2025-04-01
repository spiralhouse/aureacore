//! AureaCore service catalog

use std::path::PathBuf;
use std::process;

use aureacore::registry::{ServiceRegistry, ValidationSummary};
use clap::{Parser, Subcommand};
use tracing::{error, info};

/// Command-line arguments
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Repository URL
    #[arg(short, long, default_value = "")]
    repository: String,

    /// Git branch
    #[arg(short, long, default_value = "main")]
    branch: String,

    /// Working directory for configuration files
    #[arg(short, long, default_value = "./config")]
    work_dir: PathBuf,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Subcommands
#[derive(Subcommand)]
enum Commands {
    /// Initialize the service catalog
    Init,

    /// Update the service catalog
    Update,

    /// Validate all services
    Validate,

    /// Register a new service
    Register {
        /// Service name
        #[arg(short, long)]
        name: String,

        /// Path to the service configuration file
        #[arg(short, long)]
        config: PathBuf,
    },
}

/// Initialize the service registry
fn init_registry(cli: &Cli) -> aureacore::Result<ServiceRegistry> {
    // Use environment variables if repository URL is not provided
    let repo_url = if cli.repository.is_empty() {
        std::env::var("AUREACORE_REPO").unwrap_or_else(|_| {
            error!("Repository URL not provided. Use --repository or AUREACORE_REPO env var.");
            process::exit(1);
        })
    } else {
        cli.repository.clone()
    };

    let work_dir = cli.work_dir.clone();
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir).map_err(|e| {
            error!("Failed to create work directory: {}", e);
            aureacore::AureaCoreError::Io(e)
        })?;
    }

    ServiceRegistry::new(repo_url, cli.branch.clone(), work_dir)
}

/// Display validation summary
fn display_validation_summary(summary: &ValidationSummary) {
    println!("Validation Summary:");
    println!("------------------");
    println!("Total services: {}", summary.total_count());
    println!("Successful: {}", summary.successful_count());
    println!("Failed: {}", summary.failed_count());
    println!("Warnings: {}", summary.warning_count());
    println!("Timestamp: {}", summary.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

    if !summary.successful.is_empty() {
        println!("\nSuccessful services:");
        for service in &summary.successful {
            println!("  ✅ {}", service);
        }
    }

    if !summary.warnings.is_empty() {
        println!("\nWarnings:");
        for (service, warnings) in &summary.warnings {
            for warning in warnings {
                println!("  ⚠️  {}: {}", service, warning);
            }
        }
    }

    if !summary.failed.is_empty() {
        println!("\nFailed services:");
        for (service, error) in &summary.failed {
            println!("  ❌ {}: {}", service, error);
        }
    }
}

#[tokio::main]
async fn main() -> aureacore::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("Starting AureaCore service catalog...");

    // Parse command-line arguments
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init) => {
            info!("Initializing service catalog...");
            let mut registry = init_registry(&cli)?;
            registry.init()?;
            info!("Service catalog initialized successfully");
        }
        Some(Commands::Update) => {
            info!("Updating service catalog...");
            let mut registry = init_registry(&cli)?;
            registry.update()?;
            registry.load_services()?;
            info!("Service catalog updated successfully");
        }
        Some(Commands::Validate) => {
            info!("Validating all services...");
            let mut registry = init_registry(&cli)?;
            registry.load_services()?;

            let summary = registry.validate_all_services()?;
            display_validation_summary(&summary);

            if summary.failed_count() > 0 {
                process::exit(1);
            }
        }
        Some(Commands::Register { name, config }) => {
            info!("Registering service {}...", name);
            let mut registry = init_registry(&cli)?;

            // Read config file
            let config_content = std::fs::read_to_string(config).map_err(|e| {
                error!("Failed to read config file: {}", e);
                aureacore::AureaCoreError::Io(e)
            })?;

            // Register service
            registry.register_service(name, &config_content)?;
            info!("Service {} registered successfully", name);
        }
        None => {
            info!("No command specified, use --help for available commands");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_validation_summary() {
        let mut summary = ValidationSummary::new();
        summary.successful.push("service1".to_string());
        summary.successful.push("service2".to_string());
        summary.failed.push(("service3".to_string(), "error".to_string()));
        summary.add_warning("service1".to_string(), "minor warning".to_string());

        display_validation_summary(&summary);
    }
}
