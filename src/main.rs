mod config;
mod linker;

use clap::{Parser, Subcommand};
use config::{SkillSpec, SkillsConfig};
use std::env;
use std::path::Path;

#[derive(Parser)]
#[command(name = "skm")]
#[command(about = "Agent Skill Manager (skm) - Manage agent skills via skills.yaml", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a default skills.yaml configuration in the current directory
    Init {
        /// Optional name of the project environment (defaults to current folder name)
        #[arg(long)]
        name: Option<String>,
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
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let config_path = current_dir.join("skills.yaml");

    match command {
        Commands::Init { name } => {
            if config_path.exists() {
                return Err("skills.yaml already exists in the current directory".into());
            }
            let project_name = name.unwrap_or_else(|| {
                current_dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("my-project")
                    .to_string()
            });
            let default_config = SkillsConfig::default_init(&project_name);
            default_config.save_to_file(&config_path)?;
            println!(
                "Initialized default skills.yaml for project '{}'",
                project_name
            );
        }
        Commands::Install { global } => {
            let config = load_config(&config_path)?;
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
            println!("Listing skills for project '{}':", config.name);

            for skill in &config.skills {
                let mut status = "OK".to_string();
                let mut linked_agents = Vec::new();

                for agent in &config.agents {
                    let target_base = if global {
                        linker::get_global_agent_skills_dir(agent)
                    } else {
                        linker::get_project_agent_skills_dir(agent, &current_dir)
                    };

                    if let Some(base) = target_base {
                        let path = base.join(&skill.name);
                        if path.exists() || path.is_symlink() {
                            linked_agents.push(agent.as_str());
                        }
                    }
                }

                if linked_agents.is_empty() {
                    status = "MISSING/NOT LINKED".to_string();
                } else if linked_agents.len() < config.agents.len() {
                    status = format!("PARTIALLY LINKED ({:?})", linked_agents);
                }

                println!(" - {} (Status: {})", skill.name, status);
            }
        }
        Commands::Check { global } => {
            let config = load_config(&config_path)?;
            let mut all_ok = true;

            for skill in &config.skills {
                // Verify source path
                let source_dir = if let Some(ref local_path) = skill.path {
                    current_dir.join(local_path)
                } else {
                    let registry_name = skill.source.as_deref().unwrap_or("default");
                    let reg_path =
                        linker::resolve_registry_path(registry_name).ok_or_else(|| {
                            format!("Could not resolve path for registry: {}", registry_name)
                        })?;
                    reg_path.join(&skill.name)
                };

                if !source_dir.exists() {
                    println!(
                        "[FAIL] Skill '{}' source directory not found: {:?}",
                        skill.name, source_dir
                    );
                    all_ok = false;
                    continue;
                }

                // Verify links
                for agent in &config.agents {
                    let target_base = if global {
                        linker::get_global_agent_skills_dir(agent)
                    } else {
                        linker::get_project_agent_skills_dir(agent, &current_dir)
                    };

                    if let Some(base) = target_base {
                        let path = base.join(&skill.name);
                        if !path.exists() && !path.is_symlink() {
                            println!(
                                "[FAIL] Missing link for agent '{}' to skill '{}'",
                                agent, skill.name
                            );
                            all_ok = false;
                        }
                    }
                }
            }

            if all_ok {
                println!("[SUCCESS] All skills validated and correctly linked.");
            } else {
                return Err("Validation checks failed. Some skills or links are missing.".into());
            }
        }
    }

    Ok(())
}

fn load_config(path: &Path) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    if !path.exists() {
        return Err("skills.yaml file not found. Run 'skm init' to create one.".into());
    }
    SkillsConfig::load_from_file(path)
}

fn ensure_registries_cached(config: &SkillsConfig) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(ref registries) = config.registries {
        for (name, url) in registries {
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
    }
    Ok(())
}
