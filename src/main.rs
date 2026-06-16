mod config;
mod config_manager;
mod linker;
mod updater;
mod wizard;

use clap::{Parser, Subcommand};
use config::{SkillSpec, SkillsConfig};
use config_manager::first_time_setup;
use std::env;
use std::io::{self, Write};
use std::path::Path;
use updater::UpdateChannel;

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
        #[arg(short, long)]
        interactive: bool,
        /// Use advanced interactive wizard with more options
        #[arg(long)]
        advanced: bool,
        /// Configure for global user directory instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Install and symlink all skills specified in skills.yaml
    Install {
        /// Link skills globally (to user home directory) instead of project-local
        #[arg(short, long)]
        global: bool,
    },
    /// Add a new skill to skills.yaml and link it
    Add {
        /// Name of the skill (e.g. software-development/symphony-spec-writing)
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
}

fn main() {
    let cli = Cli::parse();

    // Check if this is first time and user is running a command that needs setup
    if config_manager::is_first_time() {
        match &cli.command {
            Commands::Setup => {}
            Commands::InitConfig => {}
            Commands::CacheUpdate { .. } => {}
            Commands::Update { .. } => {}
            _ => {
                println!("First time setup required. Running automatic setup...");
                if let Err(e) = config_manager::first_time_setup() {
                    eprintln!("Warning: First-time setup failed: {}", e);
                    eprintln!("You may need to run 'skm setup' manually.");
                }
            }
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
            interactive,
            advanced,
            global,
        } => {
            if config_path.exists() {
                return Err("skills.yaml already exists in the current directory".into());
            }

            let config = if interactive || advanced {
                if advanced {
                    // Advanced wizard is the main run_wizard
                    wizard::run_wizard(name, global)?
                } else {
                    // Use streamlined wizard for --interactive
                    wizard::run_streamlined_wizard(name, global)?
                }
            } else {
                let project_name = name.unwrap_or_else(|| {
                    current_dir
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("my-project")
                        .to_string()
                });
                SkillsConfig::default_init(&project_name)
            };

            config.save_to_file(&config_path)?;

            if global {
                println!("Initialized skills.yaml for GLOBAL user configuration");
            } else {
                let project_name = config.name;
                println!("Initialized skills.yaml for project '{}'", project_name);
            }

            // Give helpful next steps
            println!("\nNext steps:");
            if global {
                println!("  Run: skm install --global");
            } else {
                println!("  Run: skm install");
            }
            println!("  Run: skm list");
            println!("  Run: skm check");
        }
        Commands::Install { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            ensure_registries_cached(&config)?;

            println!("Installing skills for agents: {:?}", config.agents);
            for skill in &config.skills {
                linker::link_skill(skill, &current_dir, &config.agents, global)?;
            }
            println!("Successfully installed all skills.");
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
            println!("Added skill '{}' to skills.yaml", skill_name);

            ensure_registries_cached(&config)?;
            linker::link_skill(&new_skill, &current_dir, &config.agents, global)?;
        }
        Commands::List { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            println!("Listing skills for project '{}':", config.name);

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
                    println!(
                        "[FAIL] Skill '{}' source directory not found: {:?}",
                        skill.name, source_dir
                    );
                    all_ok = false;
                    continue;
                }

                if !source_dir.join("SKILL.md").exists() {
                    println!("[FAIL] Skill '{}' missing SKILL.md", skill.name);
                    all_ok = false;
                    continue;
                }

                // Verify links
                for agent in &config.agents {
                    let base = linker::get_agent_skills_dir(agent, &current_dir, global)?;
                    let path = linker::get_skill_target_path(&base, &skill.name)?;
                    if !path.is_symlink() {
                        println!(
                            "[FAIL] Missing symlink for agent '{}' to skill '{}'",
                            agent, skill.name
                        );
                        all_ok = false;
                        continue;
                    }

                    if !linker::symlink_points_to(&path, &source_dir)? {
                        println!(
                            "[FAIL] Link for agent '{}' to skill '{}' points at the wrong target",
                            agent, skill.name
                        );
                        all_ok = false;
                    }
                }
            }

            if all_ok {
                println!("[SUCCESS] All skills validated and correctly linked.");
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
                println!("Update cancelled.");
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
            config_manager::init_base_config()?;
            println!("Base configuration initialized.");
            println!("You can now use 'skm cache-update' to populate the skill registry cache.");
        }
    }

    Ok(())
}

fn confirm_update() -> Result<bool, Box<dyn std::error::Error>> {
    print!("Install this update now? [y/N] ");
    io::stdout().flush()?;

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

        println!("Cloning registry '{}' from '{}'...", name, url);
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
