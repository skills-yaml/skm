mod cleaner;
mod config;
mod config_editor;
mod config_manager;
mod dev;
mod linker;
mod registry;
mod remover;
mod updater;
mod wizard;

use clap::{Parser, Subcommand};
use config::{SkillSpec, SkillsConfig};
use config_manager::{ensure_global_env, first_time_setup};
use std::env;
use std::io::{self, Write};
use std::path::Path;
use updater::{check_and_notify_update, UpdateChannel};

#[derive(Parser)]
#[command(name = "skm")]
#[command(about = "Agent Skill Manager (skm) - Manage agent skills via skills.yaml", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a skills.yaml configuration interactively or with defaults
    Init {
        /// Optional name of the project environment (defaults to current folder name)
        #[arg(long)]
        name: Option<String>,
        /// Run in interactive mode to select skills, agents, and configuration scope
        #[arg(short, long, default_value = "true")]
        interactive: bool,
        /// Use advanced interactive wizard with more options
        #[arg(long)]
        advanced: bool,
        /// Configure for global user directory instead of project-local
        #[arg(short, long)]
        global: bool,
        /// Use non-interactive mode with default values
        #[arg(long)]
        non_interactive: bool,
    },
    /// Install and symlink all skills specified in skills.yaml
    Install {
        /// Link skills globally (to user home directory) instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Add a new skill to skills.yaml and link it
    Add {
        /// Name of the skill (e.g. software-development/spec)
        skill_name: String,
        /// Source registry name (defaults to 'default')
        #[arg(long)]
        source: Option<String>,
        /// Path to a local skill directory (for local offline skills)
        #[arg(long)]
        path: Option<String>,
        /// Link skills globally instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Remove a skill from skills.yaml and unlink it from agent directories
    Remove {
        /// Name of the skill to remove
        skill_name: String,
        /// Remove from global agent directories instead of project-local
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
        /// Remove even if target is not a symlink (use with caution)
        #[arg(long)]
        force: bool,
        /// Preview actions without making changes
        #[arg(long)]
        dry_run: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// List all defined skills and verify their current linkage status
    List {
        /// List global links status instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Verify all skills match layout expectations and are correctly linked (useful for CI)
    Check {
        /// Verify global links status instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Check for and install skm release updates
    Update {
        /// Release channel to use: prod or development
        #[arg(long, default_value = "prod")]
        channel: String,
        /// Only check whether an update is available
        #[arg(long)]
        check: bool,
        /// Install without prompting for confirmation
        #[arg(short, long)]
        yes: bool,
    },
    /// Update local cache of skill registries
    CacheUpdate {
        /// Specific registry to update (updates all if not specified)
        #[arg(long)]
        registry: Option<String>,
    },
    /// Run first-time setup (initialize base config and cache)
    Setup,
    /// Initialize global base configuration with default registry
    InitConfig,
    /// Clean up SKM artifacts (broken symlinks, cache, etc.)
    #[command(subcommand)]
    Clean(CleanCommands),
    /// Manage SKM configuration
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Manage skill registries
    #[command(subcommand)]
    Registry(RegistryCommands),
    /// Manage local development skills
    #[command(subcommand)]
    Dev(DevCommands),
}

#[derive(Subcommand)]
enum CleanCommands {
    /// Clean up broken and orphaned symlinks
    Symlinks {
        /// Clean global symlinks
        #[arg(short, long)]
        global: bool,
        /// Only clean broken symlinks
        #[arg(long)]
        broken: bool,
        /// Only clean orphaned symlinks
        #[arg(long)]
        orphaned: bool,
        /// Clean all symlinks (broken + orphaned)
        #[arg(long)]
        all: bool,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Clean up registry cache
    Cache {
        /// Clean all registry caches
        #[arg(long)]
        all: bool,
        /// Remove old skill versions
        #[arg(long)]
        old_versions: bool,
        /// Keep N most recent versions
        #[arg(short, long, default_value = "5")]
        keep: usize,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show cache statistics
        #[arg(long)]
        stats: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
        /// Specific registry to clean
        registry: Option<String>,
    },

    /// Reset SKM to clean state
    Reset {
        /// Reset configuration files
        #[arg(long)]
        config: bool,
        /// Clear all caches
        #[arg(long)]
        cache: bool,
        /// Remove all symlinks
        #[arg(long)]
        symlinks: bool,
        /// Reset everything
        #[arg(long)]
        all: bool,
        /// Create backup before reset
        #[arg(long)]
        backup: bool,
        /// Directory to store backups
        #[arg(long)]
        backup_dir: Option<String>,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get a configuration value
    Get {
        /// Configuration key (supports dot notation)
        key: String,
        /// Get from global configuration
        #[arg(short, long)]
        global: bool,
        /// Get from project configuration
        #[arg(short, long)]
        project: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show default value if key not found
        #[arg(long)]
        default: bool,
    },

    /// Set a configuration value
    Set {
        /// Configuration key
        key: String,
        /// Value to set
        value: String,
        /// Set in global configuration
        #[arg(short, long)]
        global: bool,
        /// Set in project configuration
        #[arg(short, long)]
        project: bool,
        /// Parse value as JSON
        #[arg(long)]
        json: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation for sensitive changes
        #[arg(short, long)]
        yes: bool,
    },

    /// Remove a configuration value
    Unset {
        /// Configuration key to remove
        key: String,
        /// Unset from global configuration
        #[arg(short, long)]
        global: bool,
        /// Unset from project configuration
        #[arg(short, long)]
        project: bool,
        /// Preview changes without applying
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },

    /// Show full configuration
    Show {
        /// Show global configuration
        #[arg(short, long)]
        global: bool,
        /// Show project configuration
        #[arg(short, long)]
        project: bool,
        /// Show both configurations
        #[arg(long)]
        all: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Output in YAML format
        #[arg(long)]
        yaml: bool,
        /// Show configuration file paths
        #[arg(long)]
        paths: bool,
    },

    /// Reset configuration to defaults
    Reset {
        /// Reset global configuration
        #[arg(short, long)]
        global: bool,
        /// Reset project configuration
        #[arg(short, long)]
        project: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview what would be reset
        #[arg(long)]
        dry_run: bool,
    },

    /// Validate configuration files
    Validate {
        /// Validate global configuration
        #[arg(short, long)]
        global: bool,
        /// Validate project configuration
        #[arg(short, long)]
        project: bool,
        /// Validate all configurations
        #[arg(long)]
        all: bool,
        /// Perform strict validation
        #[arg(long)]
        strict: bool,
    },
}

#[derive(Subcommand)]
enum RegistryCommands {
    /// Add a new skill registry
    Add {
        /// Name for the new registry
        name: String,
        /// Git URL of the registry
        url: String,
        /// Set this registry as the default
        #[arg(long)]
        set_default: bool,
        /// Skip URL validation
        #[arg(long)]
        skip_validate: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Remove a skill registry
    Remove {
        /// Name of the registry to remove
        name: String,
        /// Force removal even if it's the default
        #[arg(short, long)]
        force: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview what would be removed
        #[arg(long)]
        dry_run: bool,
    },

    /// List all configured registries
    List {
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Update a registry cache
    Update {
        /// Name of the registry to update
        name: Option<String>,
        /// Update all registries
        #[arg(long)]
        all: bool,
        /// Force update even if already up-to-date
        #[arg(long)]
        force: bool,
    },

    /// Set the default registry
    Default {
        /// Name of the registry to set as default
        name: String,
    },

    /// Show detailed registry information
    Info {
        /// Name of the registry
        name: String,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum DevCommands {
    /// Link a local directory as a development skill
    Link {
        /// Path to local skill directory
        path: std::path::PathBuf,
        /// Skill name (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,
        /// Registry source to override
        #[arg(short, long)]
        source: Option<String>,
        /// Link globally instead of in current project
        #[arg(short, long)]
        global: bool,
        /// Link to all available agents
        #[arg(long)]
        all_agents: bool,
        /// Link to specific agent(s)
        #[arg(long)]
        agent: Option<String>,
        /// Override existing skill without warning
        #[arg(short, long)]
        force: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Unlink a development skill
    Unlink {
        /// Name of the development skill to unlink
        skill_name: String,
        /// Unlink from global scope
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Show verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// List all linked development skills
    List {
        /// Show global development skills
        #[arg(short, long)]
        global: bool,
        /// Show both project and global
        #[arg(long)]
        all: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show full paths
        #[arg(long)]
        paths: bool,
    },

    /// Show information about a development skill
    Show {
        /// Name of the development skill
        skill_name: String,
        /// Show from global scope
        #[arg(short, long)]
        global: bool,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },

    /// Toggle development mode
    Mode {
        /// Action: on, off, or status
        action: String,
        /// Apply to global configuration
        #[arg(short, long)]
        global: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    // Always ensure global environment is configured
    if let Err(e) = ensure_global_env() {
        eprintln!("Warning: Failed to initialize global configuration: {}", e);
        eprintln!("SKM may not function correctly. Run 'skm setup' to manually configure.");
    }

    // Check for updates at launch
    if let Ok(should_update) = check_and_notify_update() {
        if should_update {
            // User agreed to update, run update and exit
            if let Err(e) = updater::install_update(UpdateChannel::Prod) {
                eprintln!("Update failed: {}", e);
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let config_path = current_dir.join("skills.yaml");

    match command {
        Commands::Init {
            name,
            advanced,
            global,
            non_interactive,
            ..
        } => {
            if config_path.exists() {
                return Err("skills.yaml already exists in the current directory".into());
            }

            let config = if non_interactive {
                // Non-interactive mode: use defaults
                let project_name = name.unwrap_or_else(|| {
                    current_dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("my-project")
                        .to_string()
                });
                SkillsConfig::default_init(&project_name)
            } else if advanced {
                // Advanced interactive wizard
                wizard::run_wizard(name, global)?
            } else {
                // Default: streamlined interactive wizard
                wizard::run_streamlined_wizard(name, global)?
            };

            config.save_to_file(&config_path)?;

            if global {
                eprintln!("Initialized skills.yaml for GLOBAL user configuration");
            } else {
                let project_name = config.name;
                eprintln!("Initialized skills.yaml for project '{}'", project_name);
            }

            // Give helpful next steps
            eprintln!("\nNext steps:");
            if global {
                eprintln!("  Run: skm install --global");
            } else {
                eprintln!("  Run: skm install");
            }
            eprintln!("  Run: skm list");
            eprintln!("  Run: skm check");
        }
        Commands::Install { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            ensure_registries_cached(&config)?;

            eprintln!("Installing skills for agents: {:?}", config.agents);
            for skill in &config.skills {
                linker::link_skill(skill, &current_dir, &config.agents, global)?;
            }
            eprintln!("Successfully installed all skills.");
        }
        Commands::Add {
            skill_name,
            source,
            path,
            global,
        } => {
            let mut config = load_config(&config_path)?;
            validate_config(&config)?;
            linker::validate_skill_name(&skill_name)?;

            // Check if skill already exists
            if config.skills.iter().any(|s| s.name == skill_name) {
                return Err(format!("Skill '{}' already exists in skills.yaml", skill_name).into());
            }

            let new_skill = SkillSpec {
                name: skill_name.clone(),
                version: Some("latest".to_string()),
                source,
                path,
            };

            config.skills.push(new_skill.clone());
            config.save_to_file(&config_path)?;
            eprintln!("Added skill '{}' to skills.yaml", skill_name);

            ensure_registries_cached(&config)?;
            linker::link_skill(&new_skill, &current_dir, &config.agents, global)?;
        }
        Commands::Remove {
            skill_name,
            global,
            yes,
            force,
            dry_run,
            verbose,
        } => {
            remover::remove_skill(
                &skill_name,
                &current_dir,
                global,
                yes,
                force,
                dry_run,
                verbose,
            )?;
        }
        Commands::List { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            eprintln!("Listing skills for project '{}':", config.name);

            for skill in &config.skills {
                let mut status = "OK".to_string();
                let mut linked_agents = Vec::new();
                let mut bad_links = Vec::new();
                let source_dir = linker::resolve_skill_source_dir(skill, &current_dir)?;
                let source_exists = source_dir.exists();

                for agent in &config.agents {
                    let base = linker::get_agent_skills_dir(agent, &current_dir, global)?;
                    let path = linker::get_skill_target_path(&base, &skill.name)?;
                    if path.is_symlink()
                        && source_exists
                        && linker::symlink_points_to(&path, &source_dir)?
                    {
                        linked_agents.push(agent.as_str());
                    } else if path.exists() || path.is_symlink() {
                        bad_links.push(agent.as_str());
                    }
                }

                if !source_exists {
                    status = "SOURCE MISSING".to_string();
                } else if !bad_links.is_empty() {
                    status = format!("BAD LINK ({:?})", bad_links);
                } else if linked_agents.is_empty() {
                    status = "MISSING/NOT LINKED".to_string();
                } else if linked_agents.len() < config.agents.len() {
                    status = format!("PARTIALLY LINKED ({:?})", linked_agents);
                }

                println!(" - {} (Status: {})", skill.name, status);
            }
        }
        Commands::Check { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            let mut all_ok = true;

            for skill in &config.skills {
                // Verify source path
                let source_dir = linker::resolve_skill_source_dir(skill, &current_dir)?;

                if !source_dir.exists() {
                    eprintln!(
                        "[FAIL] Skill '{}' source directory not found: {:?}",
                        skill.name, source_dir
                    );
                    all_ok = false;
                    continue;
                }

                if !source_dir.join("SKILL.md").exists() {
                    eprintln!("[FAIL] Skill '{}' missing SKILL.md", skill.name);
                    all_ok = false;
                    continue;
                }

                // Verify links
                for agent in &config.agents {
                    let base = linker::get_agent_skills_dir(agent, &current_dir, global)?;
                    let path = linker::get_skill_target_path(&base, &skill.name)?;
                    if !path.is_symlink() {
                        eprintln!(
                            "[FAIL] Missing symlink for agent '{}' to skill '{}'",
                            agent, skill.name
                        );
                        all_ok = false;
                        continue;
                    }

                    if !linker::symlink_points_to(&path, &source_dir)? {
                        eprintln!(
                            "[FAIL] Link for agent '{}' to skill '{}' points at the wrong target",
                            agent, skill.name
                        );
                        all_ok = false;
                    }
                }
            }

            if all_ok {
                eprintln!("[SUCCESS] All skills validated and correctly linked.");
            } else {
                return Err("Validation checks failed. Some skills or links are missing.".into());
            }
        }
        Commands::Update {
            channel,
            check,
            yes,
        } => {
            let channel = UpdateChannel::parse(&channel)?;
            let update_available = updater::check_for_update(channel)?;

            if check {
                return Ok(());
            }

            if !update_available {
                return Ok(());
            }

            if !yes && !confirm_update()? {
                eprintln!("Update cancelled.");
                return Ok(());
            }

            updater::install_update(channel)?;
        }
        Commands::CacheUpdate { registry } => {
            config_manager::update_cache(registry.as_deref())?;
        }
        Commands::Setup => {
            first_time_setup()?;
        }
        Commands::InitConfig => {
            config_manager::ensure_global_env()?;
            eprintln!("Base configuration initialized.");
            eprintln!("You can now use 'skm cache-update' to populate the skill registry cache.");
        }
        Commands::Clean(cmd) => match cmd {
            CleanCommands::Symlinks {
                global,
                broken,
                orphaned,
                all,
                dry_run,
                yes,
                verbose,
            } => {
                cleaner::clean_symlinks(global, broken, orphaned, all, dry_run, yes, verbose)?;
            }
            CleanCommands::Cache {
                all,
                old_versions,
                keep,
                dry_run,
                yes,
                stats,
                verbose,
                registry,
            } => {
                cleaner::clean_cache(
                    all,
                    old_versions,
                    keep,
                    dry_run,
                    yes,
                    stats,
                    verbose,
                    registry,
                )?;
            }
            CleanCommands::Reset {
                config,
                cache,
                symlinks,
                all,
                backup,
                backup_dir,
                dry_run,
                yes,
            } => {
                cleaner::reset(
                    config, cache, symlinks, all, backup, backup_dir, dry_run, yes,
                )?;
            }
        },
        Commands::Config(cmd) => match cmd {
            ConfigCommands::Get {
                key,
                global,
                project,
                json,
                default,
            } => {
                config_editor::get_value(&key, global, project, json, default)?;
            }
            ConfigCommands::Set {
                key,
                value,
                global,
                project,
                json,
                dry_run,
                yes,
            } => {
                config_editor::set_value(&key, &value, global, project, json, dry_run, yes)?;
            }
            ConfigCommands::Unset {
                key,
                global,
                project,
                dry_run,
                yes,
            } => {
                config_editor::unset_value(&key, global, project, dry_run, yes)?;
            }
            ConfigCommands::Show {
                global,
                project,
                all,
                json,
                yaml,
                paths,
            } => {
                config_editor::show_config(global, project, all, json, yaml, paths)?;
            }
            ConfigCommands::Reset {
                global,
                project,
                yes,
                dry_run,
            } => {
                config_editor::reset_config(global, project, yes, dry_run)?;
            }
            ConfigCommands::Validate {
                global,
                project,
                all,
                strict,
            } => {
                config_editor::validate_config(global, project, all, strict)?;
            }
        },
        Commands::Registry(cmd) => match cmd {
            RegistryCommands::Add {
                name,
                url,
                set_default,
                skip_validate,
                json,
            } => {
                registry::add(name, url, set_default, skip_validate, json)?;
            }
            RegistryCommands::Remove {
                name,
                force,
                yes,
                dry_run,
            } => {
                registry::remove(name, force, yes, dry_run)?;
            }
            RegistryCommands::List { json, verbose } => {
                registry::list(json, verbose)?;
            }
            RegistryCommands::Update { name, all, force } => {
                if all {
                    registry::update_all(force)?;
                } else if let Some(name) = name {
                    registry::update(name, force)?;
                } else {
                    return Err("Must specify a registry name or use --all".into());
                }
            }
            RegistryCommands::Default { name } => {
                registry::set_default(name)?;
            }
            RegistryCommands::Info { name, json } => {
                registry::info(name, json)?;
            }
        },
        Commands::Dev(cmd) => match cmd {
            DevCommands::Link {
                path,
                name,
                source,
                global,
                all_agents,
                agent,
                force,
                verbose,
            } => {
                dev::link_local_skill(
                    path, name, source, global, all_agents, agent, force, verbose,
                )?;
            }
            DevCommands::Unlink {
                skill_name,
                global,
                yes,
                verbose,
            } => {
                dev::unlink_local_skill(&skill_name, global, yes, verbose)?;
            }
            DevCommands::List {
                global,
                all,
                json,
                paths,
            } => {
                dev::list_local_skills(global, all, json, paths)?;
            }
            DevCommands::Show {
                skill_name,
                global,
                json,
            } => {
                dev::show_local_skill(&skill_name, global, json)?;
            }
            DevCommands::Mode { action, global } => {
                dev::toggle_dev_mode(&action, global)?;
            }
        },
    }

    Ok(())
}

fn confirm_update() -> Result<bool, Box<dyn std::error::Error>> {
    eprint!("Install this update now? [y/N] ");
    io::stderr().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn load_config(path: &Path) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err("skills.yaml file not found. Run 'skm init' to create one.".into());
    }
    SkillsConfig::load_from_file(path)
}

fn validate_config(config: &SkillsConfig) -> Result<(), Box<dyn std::error::Error>> {
    linker::validate_agents(&config.agents)?;

    for skill in &config.skills {
        linker::validate_skill_name(&skill.name)?;
    }

    Ok(())
}

fn ensure_registries_cached(config: &SkillsConfig) -> Result<(), Box<dyn std::error::Error>> {
    // First, try to use the base config registries
    let base_config = config_manager::ensure_base_config()?;

    // Merge registries from config with base config
    let mut all_registries = base_config.registries.clone();

    // Override with project-specific registries
    if let Some(ref project_registries) = config.registries {
        for (name, url) in project_registries {
            all_registries.insert(name.clone(), url.clone());
        }
    }

    for (name, url) in &all_registries {
        let path = linker::resolve_registry_path(name)
            .ok_or_else(|| format!("Could not resolve path for registry: {}", name))?;

        if path.exists() {
            continue;
        }

        // Get parent directory to clone into
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        eprintln!("Cloning registry '{}' from '{}'...", name, url);
        let output = std::process::Command::new("git")
            .args(["clone", url, path.to_str().unwrap()])
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to clone registry '{}': {}", name, err).into());
        }
    }
    Ok(())
}
