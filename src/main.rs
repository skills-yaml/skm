mod bundle;
mod cleaner;
mod config;
mod config_editor;
mod config_manager;
mod confirmation;
mod dev;
mod linker;
mod registry;
mod remover;
mod search;
mod toolkit;
mod updater;
mod version;
mod version_manager;
mod wizard;
mod workspace_source;

use clap::{Parser, Subcommand, ValueEnum};
use config::{SkillSpec, SkillsConfig};
use config_manager::{ensure_global_env, first_time_setup};
use std::env;
use std::path::Path;

#[derive(Parser)]
#[command(name = "skm")]
#[command(version)]
#[command(about = "Agent Skill Manager (skm) - Manage agent skills via skills.yaml", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the installed version and managed build identity
    Version,
    /// Create or edit skills.yaml with sequential prompts, or create defaults for scripts
    Init {
        /// Override the project name (new configurations default to the current folder name)
        #[arg(long)]
        name: Option<String>,
        /// Open the interactive prompts (the default)
        #[arg(short, long, default_value = "true")]
        interactive: bool,
        /// Compatibility alias for the complete interactive prompt flow
        #[arg(long)]
        advanced: bool,
        /// Prepare for global installation; skills.yaml stays in the current directory
        #[arg(short, long)]
        global: bool,
        /// Use non-interactive mode with default values
        #[arg(long)]
        non_interactive: bool,
        /// Select a repository-local Workspace toolkit manifest
        #[arg(long)]
        toolkit_manifest: Option<String>,
        /// Pin the selected toolkit version
        #[arg(long, default_value = "0.1.0")]
        toolkit_version: String,
        /// Select a toolkit bundle; repeat for multiple bundles
        #[arg(long)]
        bundle: Vec<String>,
        /// Select an additional role profile; repeat for multiple profiles
        #[arg(long)]
        profile: Vec<String>,
        /// Pin a workspace standard, for example workspace-docs@5.0.0
        #[arg(long)]
        workspace_standard: Option<String>,
        /// Select a repository-local workspace standard source
        #[arg(long)]
        workspace_source: Option<String>,
        /// Pin an immutable Git revision for a remote workspace source
        #[arg(long)]
        workspace_revision: Option<String>,
        /// Pin the expected SHA-256 package integrity for a remote workspace source
        #[arg(long)]
        workspace_integrity: Option<String>,
    },
    /// Install and symlink all skills specified in skills.yaml
    Install {
        /// Link skills globally (to user home directory) instead of project-local
        #[arg(short, long)]
        global: bool,
        /// Preview the complete plan without writing
        #[arg(long)]
        dry_run: bool,
        /// Emit the plan as JSON
        #[arg(long)]
        json: bool,
        /// Confirm non-interactive application
        #[arg(short, long)]
        yes: bool,
    },
    /// Add a registry skill or published bundle to this project
    Add {
        /// Skill or bundle ID (e.g. software-development/spec)
        name: String,
        /// Source registry name (defaults to 'default')
        #[arg(long)]
        source: Option<String>,
        /// Select skill or bundle explicitly when an ID names both
        #[arg(long, value_enum)]
        kind: Option<AddKind>,
        /// Path to a local skill directory (for local offline skills)
        #[arg(long)]
        path: Option<String>,
        /// Link skills globally instead of project-local
        #[arg(short, long)]
        global: bool,
        /// Preview a bundle without writing
        #[arg(long, conflicts_with = "yes")]
        dry_run: bool,
        /// Emit a bundle plan as JSON without writing
        #[arg(long, conflicts_with = "yes")]
        json: bool,
        /// Apply without prompting for confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Add every skill in a published registry bundle to this project
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },
    /// Search configured registries for skills and published bundles
    Search {
        /// Skill or bundle ID, or part of one
        query: String,
        /// Search only this configured registry
        #[arg(short, long)]
        registry: Option<String>,
        /// Emit deterministic JSON search results
        #[arg(long)]
        json: bool,
        /// Maximum number of results to display
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
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
    /// Check for or install a verified SKM release update
    Update(updater::UpdateArgs),
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
    /// List all available versions for a skill
    Versions {
        /// Name of the skill
        skill_name: String,
        /// Specific registry to query
        #[arg(short, long)]
        registry: Option<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Only show stable versions
        #[arg(long)]
        stable_only: bool,
        /// Include prerelease versions
        #[arg(long)]
        pre: bool,
        /// Limit number of versions shown
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Switch a skill to a specific version
    Use {
        /// Skill and version (format: skill@v1.2.0)
        skill_version: String,
        /// Apply to global configuration
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview changes
        #[arg(long)]
        dry_run: bool,
    },

    /// Update a skill to its latest version
    #[command(name = "update-skill")]
    UpdateSkill {
        /// Name of the skill to update
        skill_name: String,
        /// Update in global configuration
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview changes
        #[arg(long)]
        dry_run: bool,
        /// Update to prerelease version
        #[arg(long)]
        pre: bool,
    },
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
enum BundleCommands {
    /// Expand a published registry bundle into exact project skill pins
    Add {
        /// Published bundle ID in namespace/name form
        bundle: String,
        /// Configured registry containing the bundle
        #[arg(long, default_value = "default")]
        source: String,
        /// Preview all skill and link changes without writing
        #[arg(long, conflicts_with = "yes")]
        dry_run: bool,
        /// Emit a JSON plan without writing
        #[arg(long, conflicts_with = "yes")]
        json: bool,
        /// Apply the complete project change without prompting
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum AddKind {
    Skill,
    Bundle,
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
    #[cfg(windows)]
    if let Some(exit_code) = updater::run_windows_update_worker_if_requested() {
        std::process::exit(exit_code);
    }

    let cli = parse_cli_with_help_notice(env::args_os(), updater::maybe_print_startup_notice)
        .unwrap_or_else(|error| error.exit());

    // Always ensure global environment is configured
    if let Err(e) = ensure_global_env() {
        eprintln!("Warning: Failed to initialize global configuration: {}", e);
        eprintln!("SKM may not function correctly. Run 'skm setup' to manually configure.");
    }

    // Managed releases check their own channel in the background of normal commands.
    updater::maybe_print_startup_notice();

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn parse_cli_with_help_notice<I, T>(args: I, notify: impl FnOnce()) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    match Cli::try_parse_from(args) {
        Ok(cli) => Ok(cli),
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayHelp {
                notify();
            }
            Err(error)
        }
    }
}

fn run(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let config_path = current_dir.join("skills.yaml");

    match command {
        Commands::Init {
            name,
            global,
            non_interactive,
            toolkit_manifest,
            toolkit_version,
            bundle,
            profile,
            workspace_standard,
            workspace_source,
            workspace_revision,
            workspace_integrity,
            ..
        } => {
            let mut document =
                wizard::Document::load(&config_path, name.as_deref(), non_interactive)?;

            if global && toolkit_manifest.is_some() {
                return Err(
                    "toolkit initialization is project-scoped; --global is not supported".into(),
                );
            }
            if let Some(manifest) = toolkit_manifest {
                document.value["toolkit"] = serde_yaml::to_value(config::ToolkitSelection {
                    manifest,
                    version: toolkit_version,
                })?;
                if !bundle.is_empty() {
                    document.value["bundles"] = serde_yaml::to_value(bundle)?;
                }
                if !profile.is_empty() {
                    document.value["profiles"] = serde_yaml::to_value(profile)?;
                }
            } else if !bundle.is_empty() || !profile.is_empty() {
                return Err("--bundle and --profile require --toolkit-manifest".into());
            }
            if let Some(standard) = workspace_standard {
                document.value["workspace"] = serde_yaml::to_value(config::WorkspaceSelection {
                    standard,
                    source: workspace_source,
                    revision: workspace_revision,
                    integrity: workspace_integrity,
                })?;
            } else if workspace_source.is_some()
                || workspace_revision.is_some()
                || workspace_integrity.is_some()
            {
                return Err("workspace source options require --workspace-standard".into());
            }
            if non_interactive {
                document.save(global)?;
            } else if !wizard::run_wizard(&mut document, global)? {
                eprintln!("Configuration cancelled. skills.yaml was not changed.");
                return Ok(());
            }

            if global {
                eprintln!("Saved skills.yaml for GLOBAL user configuration");
            } else {
                let project_name = document.value["name"].as_str().unwrap_or_default();
                eprintln!("Saved skills.yaml for project '{}'", project_name);
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
        Commands::Install {
            global,
            dry_run,
            json,
            yes,
        } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;

            if config.toolkit.is_none() && !config.skills.is_empty() {
                linker::require_skill_targets(&config.agents, &current_dir, global)?;
            }

            if config.skills.iter().any(|skill| skill.path.is_none()) {
                ensure_registries_cached(&config)?;
            }

            if config.toolkit.is_some() {
                if global {
                    return Err(
                        "toolkit installation is project-scoped; --global is not supported".into(),
                    );
                }
                if !dry_run && !yes {
                    return Err(
                        "toolkit installation requires --yes; use --dry-run to preview".into(),
                    );
                }
                toolkit::install(
                    &config,
                    &current_dir,
                    toolkit::InstallOptions { dry_run, json },
                )?;
            } else {
                if dry_run || json {
                    return Err("--dry-run and --json require a configured toolkit".into());
                }
                eprintln!("Installing skills for agents: {:?}", config.agents);
                let resolved =
                    linker::resolve_skill_dependency_closure(&config.skills, &current_dir)?;
                linker::validate_unique_skill_targets(&resolved)?;
                for skill in &resolved {
                    linker::link_skill(skill, &current_dir, &config.agents, global)?;
                }
                eprintln!("Successfully installed all skills.");
            }
        }
        Commands::Add {
            name,
            source,
            kind,
            path,
            global,
            dry_run,
            json,
            yes,
        } => {
            add_registry_item(
                &config_path,
                &current_dir,
                &name,
                source,
                kind,
                path,
                global,
                dry_run,
                json,
                yes,
            )?;
        }
        Commands::Bundle { command } => match command {
            BundleCommands::Add {
                bundle,
                source,
                dry_run,
                json,
                yes,
            } => bundle::add(&current_dir, &bundle, &source, dry_run, json, yes)?,
        },
        Commands::Search {
            query,
            registry,
            json,
            limit,
        } => {
            if query.trim().is_empty() {
                return Err("Search query must not be empty".into());
            }
            if limit == 0 {
                return Err("--limit must be greater than zero".into());
            }
            let config = match std::fs::symlink_metadata(&config_path) {
                Ok(_) => Some(load_config(&config_path)?),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            let discovery = search::discover(config.as_ref(), &current_dir, registry.as_deref())
                .map_err(|error| format!("Could not search registries: {error}"))?;
            let matches = search::matching_items(&discovery.entries, &discovery.bundles, &query)
                .map_err(|error| format!("Could not search registries: {error}"))?;
            search::print_results(
                &query,
                &matches,
                &discovery.bundles,
                &discovery.collections,
                &discovery.warnings,
                limit,
                json,
            )?;
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
            if config.toolkit.is_none() && !config.skills.is_empty() {
                linker::require_skill_targets(&config.agents, &current_dir, global)?;
            }
            eprintln!("Listing skills for project '{}':", config.name);

            let resolved = linker::resolve_skill_dependency_closure(&config.skills, &current_dir)?;
            for skill in &resolved {
                let mut status = "OK".to_string();
                let mut linked_targets = Vec::new();
                let mut bad_targets = Vec::new();
                let source_dir = linker::resolve_skill_source_dir(skill, &current_dir)?;
                let source_exists = source_dir.exists();
                let targets =
                    linker::resolve_agent_skill_targets(&config.agents, &current_dir, global)?;

                for target in &targets {
                    let path = linker::get_skill_target_path(&target.path, &skill.name)?;
                    let label = format!("{} [{}]", target.path.display(), target.agents.join(", "));
                    if path.is_symlink()
                        && source_exists
                        && linker::symlink_points_to(&path, &source_dir)?
                    {
                        linked_targets.push(label);
                    } else if path.exists() || path.is_symlink() {
                        bad_targets.push(label);
                    }
                }

                if !source_exists {
                    status = "SOURCE MISSING".to_string();
                } else if !bad_targets.is_empty() {
                    status = format!("BAD LINK ({})", bad_targets.join(", "));
                } else if linked_targets.is_empty() && !targets.is_empty() {
                    status = "MISSING/NOT LINKED".to_string();
                } else if linked_targets.len() < targets.len() {
                    status = format!("PARTIALLY LINKED ({})", linked_targets.join(", "));
                }

                println!(" - {} (Status: {})", skill.name, status);
                for target in &targets {
                    println!(
                        "   - {} (agents: {})",
                        target.path.display(),
                        target.agents.join(", ")
                    );
                }
            }
            if config.toolkit.is_some() {
                toolkit::list(&current_dir)?;
            }
        }
        Commands::Check { global } => {
            let config = load_config(&config_path)?;
            validate_config(&config)?;
            if config.toolkit.is_none() && !config.skills.is_empty() {
                linker::require_skill_targets(&config.agents, &current_dir, global)?;
            }
            let mut all_ok = true;

            let resolved = linker::resolve_skill_dependency_closure(&config.skills, &current_dir)?;
            for skill in &resolved {
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
                for target in
                    linker::resolve_agent_skill_targets(&config.agents, &current_dir, global)?
                {
                    let path = linker::get_skill_target_path(&target.path, &skill.name)?;
                    let agents = target.agents.join(", ");
                    if !path.is_symlink() {
                        eprintln!(
                            "[FAIL] Missing symlink at '{}' for agents [{}] to skill '{}'",
                            path.display(),
                            agents,
                            skill.name
                        );
                        all_ok = false;
                        continue;
                    }

                    if !linker::symlink_points_to(&path, &source_dir)? {
                        eprintln!(
                            "[FAIL] Link at '{}' for agents [{}] to skill '{}' points at the wrong target",
                            path.display(),
                            agents,
                            skill.name
                        );
                        all_ok = false;
                    }
                }
            }

            if all_ok && config.toolkit.is_some() {
                toolkit::check(&config, &current_dir)?;
            } else if all_ok {
                eprintln!("[SUCCESS] All skills validated and correctly linked.");
            } else {
                return Err("Validation checks failed. Some skills or links are missing.".into());
            }
        }
        Commands::Version => version::show_version(),
        Commands::Update(args) => {
            println!("{}", updater::run_update(&args)?);
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
        Commands::Versions {
            skill_name,
            registry,
            json,
            stable_only,
            pre,
            limit,
        } => {
            version_manager::list_versions_cmd(
                &skill_name,
                registry.as_deref(),
                stable_only,
                pre,
                limit,
                json,
            )?;
        }
        Commands::Use {
            skill_version,
            global,
            yes,
            dry_run,
        } => {
            let (skill_name, version) = SkillSpec::parse_with_version(&skill_version)?;
            let version = version.unwrap_or_else(|| "latest".to_string());
            version_manager::use_version(
                &skill_name,
                &version,
                &config_path,
                global,
                yes,
                dry_run,
            )?;
        }
        Commands::UpdateSkill {
            skill_name,
            global,
            yes,
            dry_run,
            pre,
        } => {
            version_manager::update_to_latest(
                &skill_name,
                &config_path,
                global,
                pre,
                yes,
                dry_run,
            )?;
        }
    }

    Ok(())
}

fn load_config(path: &Path) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("skills.yaml must not be a symlink".into())
        }
        Ok(metadata) if !metadata.is_file() => {
            return Err("skills.yaml must be a regular file".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err("skills.yaml file not found. Run 'skm init' to create one.".into())
        }
        Err(error) => return Err(error.into()),
    }
    SkillsConfig::load_from_file(path)
}

fn validate_config(config: &SkillsConfig) -> Result<(), Box<dyn std::error::Error>> {
    linker::validate_agents(&config.agents)?;

    for skill in &config.skills {
        linker::validate_skill_name(&skill.name)?;
    }
    linker::validate_unique_skill_targets(&config.skills)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_registry_item(
    config_path: &Path,
    project_root: &Path,
    name: &str,
    source: Option<String>,
    kind: Option<AddKind>,
    path: Option<String>,
    global: bool,
    dry_run: bool,
    json: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    linker::validate_skill_name(name)?;
    let bundle_options = dry_run || json;
    if kind == Some(AddKind::Bundle) && path.is_some() {
        return Err("--path applies only to local skills".into());
    }
    if kind == Some(AddKind::Bundle) && global {
        return Err("published bundles can only be added to a project".into());
    }
    if path.is_none() && kind != Some(AddKind::Skill) {
        let registry = source.as_deref().unwrap_or("default");
        if kind == Some(AddKind::Bundle) {
            return bundle::add(project_root, name, registry, dry_run, json, yes);
        }
        let config = load_config(config_path)?;
        let discovery = search::discover(Some(&config), project_root, Some(registry))?;
        let skill_exists = discovery
            .entries
            .iter()
            .any(|entry| entry.name == name && entry.registry == registry);
        let bundle_exists = discovery
            .bundles
            .iter()
            .any(|bundle| bundle.id == name && bundle.registry == registry);
        if skill_exists && bundle_exists {
            return Err(format!(
                "'{name}' is both a skill and a bundle in registry '{registry}'; add --kind skill or --kind bundle"
            )
            .into());
        }
        if bundle_exists {
            if global {
                return Err("published bundles can only be added to a project".into());
            }
            return bundle::add(project_root, name, registry, dry_run, json, yes);
        }
    }
    if bundle_options {
        return Err("--dry-run and --json apply only to published bundles".into());
    }
    add_skill(
        config_path,
        project_root,
        SkillSpec {
            name: name.to_string(),
            version: Some("latest".to_string()),
            source,
            path,
        },
        global,
        yes,
    )
}

fn add_skill(
    config_path: &Path,
    project_root: &Path,
    new_skill: SkillSpec,
    global: bool,
    yes: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    add_skill_with_confirmation(
        config_path,
        project_root,
        new_skill,
        global,
        yes,
        confirmation::confirm,
    )
}

fn add_skill_with_confirmation<F>(
    config_path: &Path,
    project_root: &Path,
    new_skill: SkillSpec,
    global: bool,
    yes: bool,
    confirm: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&str) -> Result<bool, Box<dyn std::error::Error>>,
{
    let original = std::fs::read(config_path)?;
    let mut config = load_config(config_path)?;
    validate_config(&config)?;
    linker::validate_skill_name(&new_skill.name)?;
    if config
        .skills
        .iter()
        .any(|skill| skill.name == new_skill.name)
    {
        return Err(format!("Skill '{}' already exists in skills.yaml", new_skill.name).into());
    }

    let skill_name = new_skill.name.clone();
    config.skills.push(new_skill.clone());
    linker::require_skill_targets(&config.agents, project_root, global)?;
    linker::validate_unique_skill_targets(&config.skills)?;
    ensure_registries_cached(&config)?;
    let requested =
        linker::resolve_skill_dependency_closure(std::slice::from_ref(&new_skill), project_root)?;
    let resolved = linker::resolve_skill_dependency_closure(&config.skills, project_root)?;
    linker::validate_unique_skill_targets(&resolved)?;
    let link_changes = plan_skill_link_changes(&resolved, project_root, &config.agents, global)?;
    eprintln!(
        "{}",
        skill_add_summary(&new_skill, &requested, &resolved, link_changes, global)
    );
    if !yes {
        let question = format!(
            "Install {} skill{} and create or repair {link_changes} link{}?",
            resolved.len(),
            if resolved.len() == 1 { "" } else { "s" },
            if link_changes == 1 { "" } else { "s" }
        );
        if !confirm(&question)? {
            eprintln!("Add cancelled.");
            return Ok(());
        }
    }
    if std::fs::read(config_path)? != original {
        return Err("skills.yaml changed after add planning; retry".into());
    }
    config.save_to_file(config_path)?;
    eprintln!("Added skill '{}' to skills.yaml", skill_name);
    for skill in &resolved {
        linker::link_skill(skill, project_root, &config.agents, global)?;
    }
    Ok(())
}

fn skill_add_summary(
    requested: &SkillSpec,
    requested_closure: &[SkillSpec],
    resolved: &[SkillSpec],
    link_changes: usize,
    global: bool,
) -> String {
    let requested_names: std::collections::BTreeSet<_> = requested_closure
        .iter()
        .map(|skill| skill.name.as_str())
        .collect();
    let source = match &requested.path {
        Some(path) => format!("local path: {path}"),
        None => format!(
            "registry: {}",
            requested.source.as_deref().unwrap_or("default")
        ),
    };
    let mut lines = vec![
        format!("Skill: {}", requested.name),
        format!("Scope: {}", if global { "global" } else { "project" }),
        format!(
            "skills.yaml entry: {}@{} ({source})",
            requested.name,
            requested.version.as_deref().unwrap_or("latest")
        ),
        "Resolved skills to link:".to_string(),
    ];
    for skill in resolved {
        let role = if skill.name == requested.name {
            "requested"
        } else if requested_names.contains(skill.name.as_str()) {
            "dependency"
        } else {
            "already configured"
        };
        lines.push(format!(
            "  {}@{} ({role})",
            skill.name,
            skill.version.as_deref().unwrap_or("latest")
        ));
    }
    lines.push(format!("Agent links to create or repair: {link_changes}"));
    lines.join("\n")
}

fn plan_skill_link_changes(
    skills: &[SkillSpec],
    project_root: &Path,
    agents: &[String],
    global: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let targets = linker::resolve_agent_skill_targets(agents, project_root, global)?;
    let mut changes = 0;
    for skill in skills {
        let source = linker::resolve_skill_source_dir(skill, project_root)?;
        for target in &targets {
            linker::validate_skill_target_parent(&target.path, &skill.name)?;
            let path = linker::get_skill_target_path(&target.path, &skill.name)?;
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    if !linker::symlink_points_to(&path, &source)? {
                        changes += 1;
                    }
                }
                Ok(_) => {
                    return Err(format!(
                        "Refusing to replace existing non-symlink path: {}",
                        path.display()
                    )
                    .into());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => changes += 1,
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(changes)
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

#[cfg(test)]
mod help_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn help_requests_notify_before_clap_exits() {
        for args in [
            vec!["skm", "help"],
            vec!["skm", "--help"],
            vec!["skm", "update", "--help"],
        ] {
            let notified = Cell::new(false);
            let error = parse_cli_with_help_notice(args, || notified.set(true))
                .err()
                .expect("help exits through Clap");
            assert_eq!(error.kind(), clap::error::ErrorKind::DisplayHelp);
            assert!(notified.get());
        }
    }

    #[test]
    fn ordinary_commands_and_invalid_arguments_do_not_notify_during_parsing() {
        for args in [vec!["skm", "version"], vec!["skm", "--invalid"]] {
            let notified = Cell::new(false);
            let _ = parse_cli_with_help_notice(args, || notified.set(true));
            assert!(!notified.get());
        }
    }

    #[test]
    fn workspace_cli_is_removed_from_help_and_parsing() {
        let help = Cli::try_parse_from(["skm", "--help"])
            .err()
            .expect("help exits through Clap");
        assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
        assert!(!help
            .to_string()
            .lines()
            .any(|line| line.trim_start().starts_with("workspace ")));
        let removed = Cli::try_parse_from(["skm", "workspace", "audit"])
            .err()
            .expect("workspace is not a command");
        assert_eq!(removed.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn bundle_add_help_and_modes_are_project_scoped() {
        let parsed = Cli::try_parse_from([
            "skm",
            "bundle",
            "add",
            "acme/starter",
            "--source",
            "local",
            "--dry-run",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            Commands::Bundle {
                command: BundleCommands::Add { dry_run: true, .. }
            }
        ));
        assert!(Cli::try_parse_from(["skm", "bundle", "add", "acme/starter", "--global"]).is_err());
        assert!(Cli::try_parse_from([
            "skm",
            "bundle",
            "add",
            "acme/starter",
            "--yes",
            "--dry-run",
        ])
        .is_err());
        let help = Cli::try_parse_from(["skm", "bundle", "add", "--help"])
            .err()
            .expect("help exits through Clap")
            .to_string();
        for flag in ["--source", "--dry-run", "--json", "--yes"] {
            assert!(help.contains(flag));
        }
        let add_help = Cli::try_parse_from(["skm", "add", "--help"])
            .err()
            .expect("help exits through Clap")
            .to_string();
        for flag in ["--kind", "--source", "--dry-run", "--json", "--yes"] {
            assert!(add_help.contains(flag));
        }
    }
}

#[cfg(test)]
mod init_tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::path::PathBuf;

    struct Project {
        original: PathBuf,
        directory: tempfile::TempDir,
    }

    impl Project {
        fn new() -> Self {
            let original = env::current_dir().unwrap();
            let directory = tempfile::tempdir().unwrap();
            env::set_current_dir(directory.path()).unwrap();
            Self {
                original,
                directory,
            }
        }

        fn path(&self) -> PathBuf {
            self.directory.path().join("skills.yaml")
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            env::set_current_dir(&self.original).unwrap();
        }
    }

    fn init(args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
        let mut command = vec!["skm", "init"];
        command.extend_from_slice(args);
        run(Cli::try_parse_from(command)?.command)
    }

    #[test]
    #[serial]
    fn non_interactive_cli_creates_defaults_and_never_overwrites() {
        let project = Project::new();
        init(&["--non-interactive", "--name", "cli-project"]).unwrap();
        let saved = fs::read_to_string(project.path()).unwrap();
        let config = SkillsConfig::load_from_file(project.path()).unwrap();
        assert_eq!(config.name, "cli-project");
        assert_eq!(config.skills[0].name, "software-development/spec");
        assert!(init(&["--non-interactive", "--name", "overwrite"]).is_err());
        assert_eq!(fs::read_to_string(project.path()).unwrap(), saved);
    }

    #[test]
    #[serial]
    fn init_flags_are_saved_in_the_reviewed_document() {
        let project = Project::new();
        init(&[
            "--non-interactive",
            "--name",
            "toolkit-project",
            "--toolkit-manifest",
            "toolkit/manifest.yaml",
            "--toolkit-version",
            "0.2.0",
            "--bundle",
            "core",
            "--profile",
            "reviewer",
            "--workspace-standard",
            "workspace-docs@5.0.0",
            "--workspace-source",
            "workspace/standards",
        ])
        .unwrap();
        let config = SkillsConfig::load_from_file(project.path()).unwrap();
        assert_eq!(config.toolkit.unwrap().version, "0.2.0");
        assert_eq!(config.bundles, ["core"]);
        assert_eq!(config.profiles, ["reviewer"]);
        assert_eq!(
            config.workspace.unwrap().source.as_deref(),
            Some("workspace/standards")
        );
        assert!(config.trusted_sources.is_empty());
    }

    #[test]
    #[serial]
    fn invalid_flag_combinations_leave_no_manifest() {
        let project = Project::new();
        for args in [
            vec![
                "--non-interactive",
                "--global",
                "--toolkit-manifest",
                "toolkit.yaml",
            ],
            vec!["--non-interactive", "--bundle", "core"],
            vec![
                "--non-interactive",
                "--workspace-source",
                "workspace/standards",
            ],
            vec![
                "--non-interactive",
                "--trusted-source",
                "workspace/standards",
            ],
        ] {
            assert!(init(&args).is_err());
            assert!(!project.path().exists());
        }
        init(&["--non-interactive", "--global", "--name", "global-config"]).unwrap();
        assert!(project.path().exists());
    }

    #[test]
    fn interactive_aliases_parse_and_help_explains_script_mode() {
        for option in ["--interactive", "--advanced", "--global"] {
            assert!(Cli::try_parse_from(["skm", "init", option]).is_ok());
        }
        let help = match Cli::try_parse_from(["skm", "init", "--help"]) {
            Ok(_) => panic!("expected help output"),
            Err(help) => help.to_string(),
        };
        assert!(help.contains("--non-interactive"));
        assert!(help.contains("sequential prompts"));
    }
}

#[cfg(all(test, unix))]
mod search_cli_tests {
    use super::*;
    use crate::config_manager::BaseConfig;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    struct Environment {
        original_dir: PathBuf,
        original_home: Option<String>,
        original_config_home: Option<String>,
        root: tempfile::TempDir,
    }

    impl Environment {
        fn new() -> Self {
            let original_dir = env::current_dir().unwrap();
            let original_home = env::var("HOME").ok();
            let original_config_home = env::var("XDG_CONFIG_HOME").ok();
            let root = tempfile::tempdir().unwrap();
            let project = root.path().join("project");
            let home = root.path().join("home");
            let config_home = root.path().join("config");
            fs::create_dir_all(&project).unwrap();
            fs::create_dir_all(&home).unwrap();
            fs::create_dir_all(&config_home).unwrap();
            env::set_current_dir(project).unwrap();
            env::set_var("HOME", home);
            env::set_var("XDG_CONFIG_HOME", config_home);
            Self {
                original_dir,
                original_home,
                original_config_home,
                root,
            }
        }

        fn project(&self) -> PathBuf {
            self.root.path().join("project")
        }

        fn registry(&self) -> PathBuf {
            self.root.path().join("registry")
        }
    }

    impl Drop for Environment {
        fn drop(&mut self) {
            env::set_current_dir(&self.original_dir).unwrap();
            if let Some(home) = &self.original_home {
                env::set_var("HOME", home);
            } else {
                env::remove_var("HOME");
            }
            if let Some(config_home) = &self.original_config_home {
                env::set_var("XDG_CONFIG_HOME", config_home);
            } else {
                env::remove_var("XDG_CONFIG_HOME");
            }
        }
    }

    #[test]
    fn single_skill_plan_distinguishes_configured_skills_and_dependencies() {
        let requested = SkillSpec {
            name: "acme/alpha".into(),
            version: Some("latest".into()),
            source: Some("company".into()),
            path: None,
        };
        let resolved_request = SkillSpec {
            version: Some("1.0.0".into()),
            ..requested.clone()
        };
        let dependency = SkillSpec {
            name: "acme/helper".into(),
            version: Some("2.0.0".into()),
            source: Some("company".into()),
            path: None,
        };
        let existing = SkillSpec {
            name: "acme/other".into(),
            version: Some("3.0.0".into()),
            source: Some("company".into()),
            path: None,
        };
        let summary = skill_add_summary(
            &requested,
            &[resolved_request.clone(), dependency.clone()],
            &[resolved_request, dependency, existing],
            2,
            false,
        );
        assert!(summary.contains("Scope: project"));
        assert!(summary.contains("skills.yaml entry: acme/alpha@latest (registry: company)"));
        assert!(summary.contains("acme/alpha@1.0.0 (requested)"));
        assert!(summary.contains("acme/helper@2.0.0 (dependency)"));
        assert!(summary.contains("acme/other@3.0.0 (already configured)"));
        assert!(summary.contains("Agent links to create or repair: 2"));
    }

    fn git(repository: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    #[serial]
    fn search_is_read_only_and_rejects_mutation_flags() {
        assert!(Cli::try_parse_from(["skm", "search", "spec", "--add"]).is_err());
        assert!(Cli::try_parse_from(["skm", "search", "spec", "--global"]).is_err());

        let environment = Environment::new();
        let registry = environment.registry();
        let skill = registry.join("skills/software/spec/v1.2.0");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: spec\ndescription: Search fixture.\nmetadata:\n  skm-version: \"1.2.0\"\n---\n\n# Search fixture\n",
        )
        .unwrap();
        git(&registry, &["init", "--quiet"]);
        git(&registry, &["add", "."]);
        git(
            &registry,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );

        let registries =
            HashMap::from([("local".to_string(), registry.to_string_lossy().into_owned())]);
        BaseConfig {
            default_registry: "local".into(),
            registries: registries.clone(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let manifest = environment.project().join("skills.yaml");
        SkillsConfig {
            name: "search-test".into(),
            version: Some("0.1.0".into()),
            registries: Some(registries),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        }
        .save_to_file(&manifest)
        .unwrap();
        let original = fs::read(&manifest).unwrap();

        run(Commands::Search {
            query: "SPEC".into(),
            registry: Some("local".into()),
            json: false,
            limit: 50,
        })
        .unwrap();
        assert_eq!(fs::read(&manifest).unwrap(), original);
        assert!(!linker::resolve_registry_path("local").unwrap().exists());
    }

    #[test]
    #[serial]
    fn add_routes_skills_and_bundles_from_one_registry() {
        let environment = Environment::new();
        let registry = environment.registry();
        for name in ["alpha", "beta"] {
            let directory = registry.join(format!("skills/acme/{name}/v1.0.0"));
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                directory.join("SKILL.md"),
                format!("---\nname: {name}\nmetadata:\n  skm-version: 1.0.0\n---\n# {name}\n"),
            )
            .unwrap();
            std::os::unix::fs::symlink("v1.0.0", directory.parent().unwrap().join("latest"))
                .unwrap();
        }
        fs::write(
            registry.join("skills/acme/manifest.yaml"),
            "schema_version: 2\nnamespace: acme\npackages:\n  alpha: 1.0.0\n  beta: 1.0.0\nbundles:\n  starter:\n    packages: [alpha]\n",
        )
        .unwrap();
        git(&registry, &["init", "--quiet"]);
        git(&registry, &["add", "."]);
        git(
            &registry,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );

        let registries = HashMap::from([("local".into(), registry.to_string_lossy().into_owned())]);
        BaseConfig {
            default_registry: "local".into(),
            registries: registries.clone(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let config_path = environment.project().join("skills.yaml");
        SkillsConfig {
            name: "unified-add".into(),
            version: None,
            registries: Some(registries),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        }
        .save_to_file(&config_path)
        .unwrap();
        let before = fs::read(&config_path).unwrap();
        let add = |name: &str, dry_run: bool, yes: bool| Commands::Add {
            name: name.into(),
            source: Some("local".into()),
            kind: None,
            path: None,
            global: false,
            dry_run,
            json: false,
            yes,
        };

        run(add("acme/starter", true, false)).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!environment.project().join(".agents/skills/alpha").exists());
        assert!(run(add("acme/starter", false, false))
            .unwrap_err()
            .to_string()
            .contains("--yes"));
        run(add("acme/starter", false, true)).unwrap();
        let after_bundle = fs::read(&config_path).unwrap();
        run(add("acme/starter", false, true)).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), after_bundle);
        assert!(environment
            .project()
            .join(".agents/skills/alpha")
            .is_symlink());

        assert!(run(add("acme/beta", false, false))
            .unwrap_err()
            .to_string()
            .contains("--yes"));
        run(add("acme/beta", false, true)).unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 2);
        assert!(environment
            .project()
            .join(".agents/skills/beta")
            .is_symlink());
        assert!(run(add("acme/beta", true, false))
            .unwrap_err()
            .to_string()
            .contains("only to published bundles"));
    }

    #[test]
    #[serial]
    fn single_skill_confirmation_precedes_project_writes() {
        let environment = Environment::new();
        BaseConfig {
            default_registry: "default".into(),
            registries: HashMap::new(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let project = environment.project();
        let source = project.join("local-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: alpha\nmetadata:\n  skm-version: 1.0.0\n---\n# Alpha\n",
        )
        .unwrap();
        let config_path = project.join("skills.yaml");
        SkillsConfig {
            name: "confirmation".into(),
            version: None,
            registries: None,
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        }
        .save_to_file(&config_path)
        .unwrap();
        let requested = SkillSpec {
            name: "acme/alpha".into(),
            version: Some("latest".into()),
            source: None,
            path: Some("local-skill".into()),
        };
        let before = fs::read(&config_path).unwrap();
        let target = project.join(".agents/skills/alpha");

        add_skill_with_confirmation(
            &config_path,
            &project,
            requested.clone(),
            false,
            false,
            |question| {
                assert_eq!(question, "Install 1 skill and create or repair 1 link?");
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!target.exists());

        let error = add_skill(&config_path, &project, requested.clone(), false, false).unwrap_err();
        assert!(error.to_string().contains("--yes"));
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!target.exists());

        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "unmanaged").unwrap();
        let error = add_skill_with_confirmation(
            &config_path,
            &project,
            requested.clone(),
            false,
            false,
            |_| panic!("a link collision must fail before confirmation"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("non-symlink path"));
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert_eq!(fs::read(&target).unwrap(), b"unmanaged");
        fs::remove_file(&target).unwrap();

        add_skill_with_confirmation(&config_path, &project, requested, false, false, |_| {
            Ok(true)
        })
        .unwrap();
        assert_eq!(
            SkillsConfig::load_from_file(&config_path)
                .unwrap()
                .skills
                .len(),
            1
        );
        assert!(target.is_symlink());
    }

    #[test]
    #[serial]
    fn add_rejects_skill_bundle_id_collision_before_writes() {
        let environment = Environment::new();
        let registry = environment.registry();
        let skill = registry.join("skills/acme/starter/v1.0.0");
        fs::create_dir_all(&skill).unwrap();
        std::os::unix::fs::symlink("v1.0.0", skill.parent().unwrap().join("latest")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: starter\n---\n# Starter\n",
        )
        .unwrap();
        fs::write(
            registry.join("skills/acme/manifest.yaml"),
            "schema_version: 2\nnamespace: acme\npackages:\n  starter: 1.0.0\nbundles:\n  starter:\n    packages: [starter]\n",
        )
        .unwrap();
        git(&registry, &["init", "--quiet"]);
        git(&registry, &["add", "."]);
        git(
            &registry,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-qm",
                "fixture",
            ],
        );
        let registries = HashMap::from([("local".into(), registry.to_string_lossy().into_owned())]);
        BaseConfig {
            default_registry: "local".into(),
            registries: registries.clone(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let config_path = environment.project().join("skills.yaml");
        SkillsConfig {
            name: "collision".into(),
            version: None,
            registries: Some(registries),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        }
        .save_to_file(&config_path)
        .unwrap();
        let before = fs::read(&config_path).unwrap();
        let error = add_registry_item(
            &config_path,
            &environment.project(),
            "acme/starter",
            Some("local".into()),
            None,
            None,
            false,
            false,
            false,
            true,
        )
        .unwrap_err();
        assert!(error.to_string().contains("both a skill and a bundle"));
        assert_eq!(fs::read(&config_path).unwrap(), before);
        add_registry_item(
            &config_path,
            &environment.project(),
            "acme/starter",
            Some("local".into()),
            Some(AddKind::Bundle),
            None,
            false,
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        add_registry_item(
            &config_path,
            &environment.project(),
            "acme/starter",
            Some("local".into()),
            Some(AddKind::Skill),
            None,
            false,
            false,
            false,
            true,
        )
        .unwrap();
        assert_eq!(
            SkillsConfig::load_from_file(&config_path)
                .unwrap()
                .skills
                .len(),
            1
        );
    }

    #[test]
    #[serial]
    fn install_requires_targets_and_links_namespaced_skills_at_discoverable_depth() {
        let environment = Environment::new();
        let project = environment.project();
        let source = project.join("fixture");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("SKILL.md"),
            "---\nname: wk-spec\ndescription: Write a spec.\n---\n# Spec\n",
        )
        .unwrap();
        let config_path = project.join("skills.yaml");
        let mut config = SkillsConfig {
            name: "fixture".into(),
            version: None,
            registries: None,
            agents: Vec::new(),
            skills: vec![SkillSpec {
                name: "workspace/wk-spec".into(),
                version: None,
                source: None,
                path: Some("fixture".into()),
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            workspace: None,
            trusted_sources: Vec::new(),
        };
        config.save_to_file(&config_path).unwrap();
        let install = || Commands::Install {
            global: false,
            dry_run: false,
            json: false,
            yes: false,
        };
        assert!(run(install())
            .unwrap_err()
            .to_string()
            .contains("No agent skill targets"));
        assert!(!project.join(".agents/skills").exists());
        config.agents.push("codex".into());
        config.save_to_file(&config_path).unwrap();
        run(install()).unwrap();
        assert!(project.join(".agents/skills/wk-spec").is_symlink());
        assert!(!project.join(".agents/skills/workspace/wk-spec").exists());
        run(Commands::Check { global: false }).unwrap();
    }
}
