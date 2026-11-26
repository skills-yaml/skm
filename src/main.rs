mod cleaner;
mod config;
mod config_editor;
mod config_manager;
mod confirmation;
mod dev;
mod installer;
mod linker;
mod registry;
mod remover;
mod search;
mod toolkit;
mod updater;
mod version;
mod version_manager;
mod wizard;

use clap::{Parser, Subcommand, ValueEnum};
use config::{SkillSpec, SkillsConfig};
use config_manager::{ensure_global_env, first_time_setup};
use std::env;
use std::ffi::{OsStr, OsString};
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
    /// Create or edit skills.yaml with sequential prompts, or create defaults for scripts
    Init {
        /// Override the project name (new configurations default to the current folder name)
        #[arg(long)]
        name: Option<String>,
        /// Open the interactive prompts (the default)
        #[arg(short, long, default_value = "true", hide = true)]
        interactive: bool,
        /// Compatibility alias for the complete interactive prompt flow
        #[arg(long, hide = true)]
        advanced: bool,
        /// Edit the user-wide skills manifest in ~/.skm
        #[arg(short, long)]
        global: bool,
        /// Use non-interactive mode with default values
        #[arg(long)]
        non_interactive: bool,
        /// Select a repository-local toolkit manifest
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
        /// Registry to add from (defaults to the configured default registry)
        #[arg(long = "registry", alias = "source", value_name = "REGISTRY")]
        source: Option<String>,
        /// Select skill or bundle explicitly when an ID names both
        #[arg(long, value_enum)]
        kind: Option<AddKind>,
        /// Path to a local skill directory (for local offline skills)
        #[arg(long)]
        path: Option<String>,
        /// Select the global installation scope (bundles require project scope)
        #[arg(short, long)]
        global: bool,
        /// Preview the manifest entry without writing
        #[arg(long, conflicts_with = "yes")]
        dry_run: bool,
        /// Emit the planned manifest entry as JSON without writing
        #[arg(long, conflicts_with = "yes")]
        json: bool,
        /// Apply without prompting for confirmation
        #[arg(long)]
        yes: bool,
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
    /// Manage the local registry cache
    #[command(subcommand)]
    Cache(CacheCommands),
    /// Inspect and change configured skill versions
    #[command(subcommand)]
    Skill(SkillCommands),
    /// Inspect and update the SKM binary
    #[command(name = "self")]
    #[command(subcommand)]
    SelfUpdate(SelfCommands),
    /// Run first-time setup (initialize base config and cache)
    Setup,
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
enum CacheCommands {
    /// Fetch current contents of one registry or all configured registries
    Refresh {
        /// Registry to refresh (all when omitted)
        registry: Option<String>,
    },
    /// Show local registry cache statistics
    Status {
        /// Registry to inspect (all when omitted)
        registry: Option<String>,
    },
    /// Remove old, unprotected cached skill versions
    Prune {
        /// Registry to prune
        #[arg(required_unless_present = "all", conflicts_with = "all")]
        registry: Option<String>,
        /// Prune all configured registries
        #[arg(long)]
        all: bool,
        /// Keep this many recent versions per skill
        #[arg(long, default_value_t = 5)]
        keep: usize,
        /// Preview removals without writing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
    /// Remove selected registry cache directories
    Clear {
        /// Registry to clear
        #[arg(required_unless_present = "all", conflicts_with = "all")]
        registry: Option<String>,
        /// Clear all configured registry caches
        #[arg(long)]
        all: bool,
        /// Preview removals without writing
        #[arg(long)]
        dry_run: bool,
        /// Skip confirmation
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum SkillCommands {
    /// List versions available in a cached registry
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
        #[arg(short, long, default_value_t = 50)]
        limit: usize,
    },
    /// Switch a skill to a specific cached version
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
    /// Show configured skills with newer cached versions
    Outdated {
        /// Inspect the user-wide skills manifest
        #[arg(short, long)]
        global: bool,
        /// Skill to inspect (all configured skills when omitted)
        skill_name: Option<String>,
        /// Refresh relevant registries before comparing versions
        #[arg(long)]
        refresh: bool,
        /// Include prerelease versions
        #[arg(long)]
        pre: bool,
    },
    /// Change a skill pin and links to the newest cached version
    Upgrade {
        /// Name of the skill to upgrade (omit with --all)
        #[arg(required_unless_present = "all", conflicts_with = "all")]
        skill_name: Option<String>,
        /// Upgrade every outdated pinned registry skill
        #[arg(long)]
        all: bool,
        /// Update in global configuration
        #[arg(short, long)]
        global: bool,
        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
        /// Preview changes
        #[arg(long)]
        dry_run: bool,
        /// Include prerelease versions
        #[arg(long)]
        pre: bool,
    },
}

#[derive(Subcommand)]
enum SelfCommands {
    /// Show the installed version and managed build identity
    Version,
    /// Check the selected release channel for a newer SKM binary
    Check {
        /// Release channel to check
        #[arg(long, value_enum)]
        channel: Option<updater::UpdateChannel>,
    },
    /// Install a verified SKM release update
    Upgrade {
        /// Release channel to follow
        #[arg(long, value_enum)]
        channel: Option<updater::UpdateChannel>,
        /// Confirm a non-interactive update
        #[arg(long)]
        yes: bool,
    },
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
        #[arg(short, long, hide = true, conflicts_with = "global")]
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
        #[arg(short, long, hide = true, conflicts_with = "global")]
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
        #[arg(short, long, hide = true, conflicts_with = "global")]
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

    /// Set the default registry
    #[command(name = "set-default", alias = "default")]
    SetDefault {
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
}

fn main() {
    #[cfg(windows)]
    if let Some(exit_code) = updater::run_windows_update_worker_if_requested() {
        std::process::exit(exit_code);
    }

    let args: Vec<OsString> = env::args_os().collect();
    if is_legacy_updater_version_probe(&args, env::var_os("SKM_NO_UPDATE_CHECK").as_deref()) {
        version::show_version();
        return;
    }

    let cli = parse_cli_with_help_notice(args, updater::maybe_print_startup_notice)
        .unwrap_or_else(|error| error.exit());

    // Preview and identity operations must leave user state untouched.
    let read_only_startup = matches!(
        &cli.command,
        Commands::Install { dry_run: true, .. }
            | Commands::Install { json: true, .. }
            | Commands::Add { dry_run: true, .. }
            | Commands::Add { json: true, .. }
            | Commands::Remove { dry_run: true, .. }
            | Commands::Cache(
                CacheCommands::Prune { dry_run: true, .. }
                    | CacheCommands::Clear { dry_run: true, .. }
            )
            | Commands::Clean(
                CleanCommands::Symlinks { dry_run: true, .. }
                    | CleanCommands::Reset { dry_run: true, .. }
            )
            | Commands::Config(
                ConfigCommands::Set { dry_run: true, .. }
                    | ConfigCommands::Unset { dry_run: true, .. }
                    | ConfigCommands::Reset { dry_run: true, .. }
            )
            | Commands::Registry(RegistryCommands::Remove { dry_run: true, .. })
            | Commands::Skill(
                SkillCommands::Use { dry_run: true, .. }
                    | SkillCommands::Upgrade { dry_run: true, .. }
            )
            | Commands::SelfUpdate(SelfCommands::Version)
    );
    let recovery = matches!(
        &cli.command,
        Commands::Config(ConfigCommands::Reset { global: true, .. })
            | Commands::Config(ConfigCommands::Reset {
                global: false,
                project: false,
                ..
            })
            | Commands::Clean(
                CleanCommands::Reset { config: true, .. } | CleanCommands::Reset { all: true, .. }
            )
    );
    if !read_only_startup && !recovery {
        if let Err(e) = ensure_global_env() {
            eprintln!("Error: cannot initialize SKM user state: {e}");
            std::process::exit(1);
        }
    }

    // Managed releases check their own channel in the background of normal commands.
    if !read_only_startup {
        updater::maybe_print_startup_notice();
    }

    if let Err(e) = run(cli.command) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn is_legacy_updater_version_probe(args: &[OsString], no_update_check: Option<&OsStr>) -> bool {
    no_update_check == Some(OsStr::new("1"))
        && args.len() == 2
        && args[1].as_os_str() == OsStr::new("version")
}

fn parse_cli_with_help_notice<I, T>(args: I, notify: impl FnOnce()) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let bare_invocation = args.len() == 1;
    match Cli::try_parse_from(args) {
        Ok(cli) => Ok(cli),
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayHelp
                || (bare_invocation
                    && error.kind()
                        == clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand)
            {
                notify();
            }
            Err(error)
        }
    }
}

struct ManifestScope {
    root: std::path::PathBuf,
    path: std::path::PathBuf,
}

impl ManifestScope {
    fn select(global: bool, current_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = if global {
            config_manager::check_global_manifest_parent()?;
            config_manager::get_global_manifest_path()
                .ok_or("Could not determine user home directory")?
        } else {
            current_dir.join("skills.yaml")
        };
        let root = path
            .parent()
            .ok_or("Manifest has no parent directory")?
            .to_path_buf();
        Ok(Self { root, path })
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
            ..
        } => {
            if global && toolkit_manifest.is_some() {
                return Err(
                    "toolkit initialization is project-scoped; --global is not supported".into(),
                );
            }
            let scope = ManifestScope::select(global, &current_dir)?;
            let mut document = wizard::Document::load_for_scope(
                &scope.path,
                name.as_deref(),
                non_interactive,
                global,
            )?;
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
                eprintln!("  Run: skm list --global");
                eprintln!("  Run: skm check --global");
            } else {
                eprintln!("  Run: skm install");
                eprintln!("  Run: skm list");
                eprintln!("  Run: skm check");
            }
        }
        Commands::Install {
            global,
            dry_run,
            json,
            yes,
        } => {
            let scope = ManifestScope::select(global, &current_dir)?;
            let config = load_config_for_scope(&scope.path, global)?;
            validate_config(&config)?;
            validate_manifest_scope(&config, global)?;

            if config.toolkit.is_none() && !config.skills.is_empty() {
                linker::require_skill_targets(&config.agents, &scope.root, global)?;
            }

            if config.skills.iter().any(|skill| skill.path.is_none()) {
                validate_effective_registry_caches(&config, &scope.root)?;
                if !dry_run && !json {
                    ensure_registries_cached(&config, &scope.root)?;
                }
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
                    &scope.root,
                    toolkit::InstallOptions { dry_run, json },
                )?;
            } else {
                let plan = installer::prepare(&config, &scope.root, global, None)?;
                plan.print(json)?;
                if !dry_run && !json {
                    plan.apply()?;
                    eprintln!("Successfully installed all skills.");
                }
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
            let scope = ManifestScope::select(global, &current_dir)?;
            let path = match (global, path) {
                (true, Some(path)) => {
                    let path = Path::new(&path);
                    let absolute = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        current_dir.join(path)
                    };
                    Some(
                        absolute
                            .to_str()
                            .ok_or("Local path is not valid UTF-8")?
                            .to_owned(),
                    )
                }
                (_, path) => path,
            };
            add_registry_item(
                &scope.path,
                &scope.root,
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
            let scope = ManifestScope::select(global, &current_dir)?;
            if global {
                validate_manifest_scope(&load_config_for_scope(&scope.path, global)?, true)?;
            }
            let config_path = &scope.path;
            let original = std::fs::read(config_path).ok();
            let had_bundle = original
                .as_deref()
                .and_then(|bytes| serde_yaml::from_slice::<SkillsConfig>(bytes).ok())
                .is_some_and(|config| config.skills.iter().any(|skill| skill.is_bundle()));
            remover::remove_skill(
                &skill_name,
                &scope.root,
                global,
                yes,
                force,
                dry_run,
                verbose,
            )?;
            if !global && !dry_run && !had_bundle && std::fs::read(config_path).ok() != original {
                installer::sync_existing_lock(config_path, &current_dir)?;
            }
        }
        Commands::List { global } => {
            let scope = ManifestScope::select(global, &current_dir)?;
            let config = load_config_for_scope(&scope.path, global)?;
            validate_config(&config)?;
            validate_manifest_scope(&config, global)?;
            if config.toolkit.is_none() && !config.skills.is_empty() {
                linker::require_skill_targets(&config.agents, &scope.root, global)?;
            }
            eprintln!(
                "Listing skills for {} '{}':",
                if global { "global manifest" } else { "project" },
                config.name
            );

            let entries = linker::resolve_entries(&config.skills, &scope.root)?;
            // Print bundle entries first with their member count.
            for entry in &entries {
                if let linker::ResolvedEntry::Bundle { spec, members, .. } = entry {
                    println!(
                        " - {} (bundle, {} members, version: {}, source: {})",
                        spec.name,
                        members.len(),
                        spec.version.as_deref().unwrap_or("latest"),
                        spec.source.as_deref().unwrap_or("default")
                    );
                }
            }
            let resolved = linker::resolve_skill_dependency_closure(&config.skills, &scope.root)?;
            linker::validate_unique_skill_targets(&resolved)?;
            for skill in &resolved {
                let mut status = "OK".to_string();
                let mut linked_targets = Vec::new();
                let mut bad_targets = Vec::new();
                let source_dir = linker::resolve_skill_source_dir(skill, &scope.root)?;
                let source_exists = source_dir.exists();
                let targets =
                    linker::resolve_agent_skill_targets(&config.agents, &scope.root, global)?;

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
                toolkit::list(&scope.root)?;
            }
        }
        Commands::Check { global } => {
            let scope = ManifestScope::select(global, &current_dir)?;
            check_skills(&scope.path, &scope.root, global)?;
        }
        Commands::SelfUpdate(command) => match command {
            SelfCommands::Version => version::show_version(),
            SelfCommands::Check { channel } => {
                let args = updater::UpdateArgs {
                    channel,
                    check: true,
                    yes: false,
                };
                println!("{}", updater::run_update(&args)?);
            }
            SelfCommands::Upgrade { channel, yes } => {
                let args = updater::UpdateArgs {
                    channel,
                    check: false,
                    yes,
                };
                println!("{}", updater::run_update(&args)?);
            }
        },
        Commands::Cache(command) => match command {
            CacheCommands::Refresh { registry } => {
                let config = if config_path.exists() {
                    Some(load_config(&config_path)?)
                } else {
                    None
                };
                registry::refresh_effective(config.as_ref(), registry.as_deref())?;
            }
            CacheCommands::Status { registry } => cleaner::show_cache_stats(registry)?,
            CacheCommands::Prune {
                registry,
                all,
                keep,
                dry_run,
                yes,
            } => cleaner::clean_cache(all, true, keep, dry_run, yes, false, false, registry)?,
            CacheCommands::Clear {
                registry,
                all,
                dry_run,
                yes,
            } => cleaner::clean_cache(all, false, 5, dry_run, yes, false, false, registry)?,
        },
        Commands::Setup => {
            first_time_setup()?;
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
            RegistryCommands::SetDefault { name } => {
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
        },
        Commands::Skill(SkillCommands::Versions {
            skill_name,
            registry,
            json,
            stable_only,
            pre,
            limit,
        }) => {
            version_manager::list_versions_cmd(
                &skill_name,
                registry.as_deref(),
                stable_only,
                pre,
                limit,
                json,
            )?;
        }
        Commands::Skill(SkillCommands::Use {
            skill_version,
            global,
            yes,
            dry_run,
        }) => {
            let scope = ManifestScope::select(global, &current_dir)?;
            if global {
                validate_manifest_scope(&load_config_for_scope(&scope.path, global)?, true)?;
            }
            let config_path = &scope.path;
            let original = std::fs::read(config_path).ok();
            let (skill_name, version) = SkillSpec::parse_with_version(&skill_version)?;
            let version = version.unwrap_or_else(|| "latest".to_string());
            version_manager::use_version(&skill_name, &version, config_path, global, yes, dry_run)?;
            if !global && !dry_run && std::fs::read(config_path).ok() != original {
                installer::sync_existing_lock(config_path, &current_dir)?;
            }
        }
        Commands::Skill(SkillCommands::Outdated {
            global,
            skill_name,
            refresh,
            pre,
        }) => {
            let scope = ManifestScope::select(global, &current_dir)?;
            if global {
                validate_manifest_scope(&load_config_for_scope(&scope.path, global)?, true)?;
            }
            if global && refresh {
                return Err("Global outdated refresh is not yet supported; from ~/.skm run `skm cache refresh NAME`, then retry without --refresh".into());
            }
            version_manager::show_outdated(&scope.path, skill_name.as_deref(), refresh, pre)?;
        }
        Commands::Skill(SkillCommands::Upgrade {
            skill_name,
            all,
            global,
            yes,
            dry_run,
            pre,
        }) => {
            let scope = ManifestScope::select(global, &current_dir)?;
            if global {
                validate_manifest_scope(&load_config_for_scope(&scope.path, global)?, true)?;
            }
            let config_path = &scope.path;
            if all {
                version_manager::upgrade_all(config_path, global, pre, yes, dry_run)?;
            } else if let Some(skill_name) = skill_name {
                let original = std::fs::read(config_path).ok();
                version_manager::update_to_latest(
                    &skill_name,
                    config_path,
                    global,
                    pre,
                    yes,
                    dry_run,
                )?;
                if !global && !dry_run && std::fs::read(config_path).ok() != original {
                    installer::sync_existing_lock(config_path, &current_dir)?;
                }
            }
        }
    }

    Ok(())
}

fn validate_manifest_scope(
    config: &SkillsConfig,
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if global
        && (config.toolkit.is_some()
            || !config.bundles.is_empty()
            || !config.profiles.is_empty()
            || config.skills.iter().any(SkillSpec::is_bundle))
    {
        return Err("Toolkits and published bundles are project-only".into());
    }
    Ok(())
}

fn load_config_for_scope(
    path: &Path,
    global: bool,
) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    if global
        && matches!(std::fs::symlink_metadata(path), Err(ref error) if error.kind() == std::io::ErrorKind::NotFound)
    {
        return Err("skills.yaml file not found. Run 'skm init --global' to create one.".into());
    }
    load_config(path)
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

fn check_skills(
    config_path: &Path,
    current_dir: &Path,
    global: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_config_for_scope(config_path, global)?;
    validate_config(&config)?;
    validate_manifest_scope(&config, global)?;
    if config.toolkit.is_none() && !config.skills.is_empty() {
        linker::require_skill_targets(&config.agents, current_dir, global)?;
    }
    let mut all_ok = true;

    let resolved = linker::resolve_skill_dependency_closure(&config.skills, current_dir)?;
    linker::validate_unique_skill_targets(&resolved)?;
    for skill in &resolved {
        // Verify source path
        let source_dir = linker::resolve_skill_source_dir(skill, current_dir)?;

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
        for target in linker::resolve_agent_skill_targets(&config.agents, current_dir, global)? {
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
                    "[FAIL] Link at '{}' for agents [{}] to skill '{}' points at the wrong target; run `skm install{}` to reconcile it",
                    path.display(),
                    agents,
                    skill.name,
                    if global { " --global" } else { "" }
                );
                all_ok = false;
            }
        }
    }

    if all_ok && config.toolkit.is_some() {
        toolkit::check(&config, current_dir)?;
    } else if all_ok {
        if !global {
            installer::verify_lock(&config, current_dir)?;
        }
        eprintln!("[SUCCESS] All skills validated and correctly linked.");
    } else {
        return Err("Validation checks failed. Some skills or links are missing.".into());
    }

    Ok(())
}

fn validate_config(config: &SkillsConfig) -> Result<(), Box<dyn std::error::Error>> {
    linker::validate_agents(&config.agents)?;

    for skill in &config.skills {
        linker::validate_skill_name(&skill.name)?;
        skill.validate_kind()?;
        if skill.is_bundle() && skill.path.is_some() {
            return Err(format!(
                "Skill '{}' is kind bundle and cannot use a local path",
                skill.name
            )
            .into());
        }
    }
    linker::validate_configured_skill_targets(&config.skills)?;

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
    if kind == Some(AddKind::Bundle) && path.is_some() {
        return Err("--path applies only to local skills".into());
    }
    if kind == Some(AddKind::Bundle) && global {
        return Err("published bundles can only be added to a project".into());
    }
    let config = load_config_for_scope(config_path, global)?;
    validate_manifest_scope(&config, global)?;
    let source = if path.is_none() {
        Some(source.unwrap_or(config_manager::BaseConfig::load()?.default_registry))
    } else {
        source
    };
    let mut is_bundle = false;
    if path.is_none() {
        let registry = source.as_deref().unwrap_or("default");
        let (discovery, root, _temporary) =
            search::discover_for_add(&config, project_root, registry, dry_run || json)?;
        let skill_exists = discovery.entries.iter().any(|entry| entry.name == name);
        let bundle_exists = discovery.bundles.iter().any(|bundle| bundle.id == name);
        if kind.is_none() && skill_exists && bundle_exists {
            return Err(format!("'{name}' is both a skill and a bundle in registry '{registry}'; add --kind skill or --kind bundle").into());
        }
        is_bundle = kind == Some(AddKind::Bundle) || (kind.is_none() && bundle_exists);
        if is_bundle {
            if global {
                return Err("published bundles can only be added to a project".into());
            }
            let (namespace, id) = name
                .split_once('/')
                .ok_or("Bundle ID must be namespace/name")?;
            if id.contains('/') {
                return Err("Bundle ID must be namespace/name".into());
            }
            linker::published_members(&root, namespace, id, registry, None)?;
        } else if !skill_exists {
            return Err(format!("Skill '{name}' is not published by registry '{registry}'").into());
        }
    }
    let entry = SkillSpec {
        name: name.to_string(),
        version: Some("latest".into()),
        source,
        path,
        kind: is_bundle.then(|| "bundle".to_string()),
    };
    add_skill_with_confirmation(
        config_path,
        entry,
        global,
        dry_run,
        json,
        yes,
        confirmation::confirm,
    )
}

#[allow(clippy::too_many_arguments)]
fn add_skill_with_confirmation<F>(
    config_path: &Path,
    new_skill: SkillSpec,
    global: bool,
    dry_run: bool,
    json: bool,
    yes: bool,
    confirm: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnOnce(&str) -> Result<bool, Box<dyn std::error::Error>>,
{
    let config = load_config_for_scope(config_path, global)?;
    validate_manifest_scope(&config, global)?;
    let original = std::fs::read(config_path)?;
    let mut config = load_config_for_scope(config_path, global)?;
    if std::fs::read(config_path)? != original {
        return Err("skills.yaml changed after add planning; retry".into());
    }
    validate_manifest_scope(&config, global)?;
    validate_config(&config)?;
    linker::validate_skill_name(&new_skill.name)?;
    new_skill.validate_kind()?;
    let existing = config
        .skills
        .iter()
        .find(|skill| skill.name == new_skill.name);
    let already_present = if let Some(skill) = existing {
        if skill.is_bundle() == new_skill.is_bundle()
            && skill.source == new_skill.source
            && skill.path == new_skill.path
        {
            true
        } else {
            return Err(format!(
                "Entry '{}' already exists in skills.yaml with a different kind or source",
                new_skill.name
            )
            .into());
        }
    } else {
        false
    };
    if !already_present {
        config.skills.push(new_skill.clone());
        linker::validate_configured_skill_targets(&config.skills)?;
    }
    if json {
        println!(
            "{}",
            format_add_preview(&new_skill, global, already_present)?
        );
    } else if already_present {
        eprintln!("'{}' is already present in skills.yaml", new_skill.name);
    } else {
        eprintln!(
            "Add {} '{}' to skills.yaml ({} scope); run skm install{} afterwards.",
            if new_skill.is_bundle() {
                "bundle"
            } else {
                "skill"
            },
            new_skill.name,
            if global { "global" } else { "project" },
            if global { " --global" } else { "" }
        );
    }
    if dry_run || json || already_present {
        return Ok(());
    }
    if !yes && !confirm("Add this entry to skills.yaml?")? {
        eprintln!("Add cancelled.");
        return Ok(());
    }
    if std::fs::read(config_path)? != original {
        return Err("skills.yaml changed after add planning; retry".into());
    }
    let mut document: serde_yaml::Value = serde_yaml::from_slice(&original)?;
    let entries = document
        .as_mapping_mut()
        .ok_or("skills.yaml must be a YAML mapping")?
        .entry(serde_yaml::Value::String("skills".into()))
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));
    entries
        .as_sequence_mut()
        .ok_or("skills must be a YAML sequence")?
        .push(serde_yaml::to_value(&new_skill)?);
    let updated = serde_yaml::to_string(&document)?;
    use std::io::Write;
    let mut staged =
        tempfile::NamedTempFile::new_in(config_path.parent().unwrap_or(Path::new(".")))?;
    staged
        .as_file()
        .set_permissions(std::fs::metadata(config_path)?.permissions())?;
    staged.write_all(updated.as_bytes())?;
    staged.as_file().sync_all()?;
    if std::fs::read(config_path)? != original {
        return Err("skills.yaml changed after add planning; retry".into());
    }
    staged.persist(config_path)?;
    eprintln!(
        "Added '{}' to skills.yaml; run skm install{} to link it.",
        new_skill.name,
        if global { " --global" } else { "" }
    );
    Ok(())
}

fn format_add_preview(
    entry: &SkillSpec,
    global: bool,
    already_present: bool,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&serde_json::json!({
        "entry": entry, "scope": if global { "global" } else { "project" },
        "already_present": already_present
    }))
}

fn validate_effective_registry_caches(
    config: &SkillsConfig,
    project: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut registries = config_manager::BaseConfig::load()?.registries;
    if let Some(overrides) = &config.registries {
        registries.extend(overrides.clone());
    }
    for name in config
        .skills
        .iter()
        .filter(|skill| skill.path.is_none())
        .map(|skill| skill.source.as_deref().unwrap_or("default"))
    {
        let location = registries
            .get(name)
            .ok_or_else(|| format!("Registry '{name}' is not configured"))?;
        linker::validate_registry_cache_origin(name, location, project)?;
    }
    Ok(())
}

fn ensure_registries_cached(
    config: &SkillsConfig,
    project: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
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

    // Validate every existing candidate before any clone or symlink is written.
    let paths = all_registries
        .iter()
        .map(|(name, url)| {
            linker::validated_registry_path(name, url, project).map(|path| (name, url, path))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (name, url, selected) in paths {
        if std::fs::symlink_metadata(&selected).is_ok() {
            continue;
        }
        let path = linker::canonical_registry_path(name)
            .ok_or_else(|| format!("Invalid registry name: {name}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let local = project.join(url);
        if local.is_dir() && !crate::linker::is_git_repository(&local) {
            linker::symlink_dir(&std::fs::canonicalize(local)?, &path)?;
            continue;
        }
        eprintln!("Cloning registry '{}' from '{}'...", name, url);
        let output = std::process::Command::new("git")
            .arg("clone")
            .current_dir(project)
            .arg("--")
            .arg(url)
            .arg(&path)
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
    fn bare_invocation_notifies_without_changing_clap_help_behavior() {
        let original = Cli::try_parse_from(["skm"]).err().expect("bare help");
        let notified = Cell::new(false);
        let error = parse_cli_with_help_notice(["skm"], || notified.set(true))
            .err()
            .expect("bare help exits through Clap");
        assert!(notified.get());
        assert_eq!(error.kind(), original.kind());
        assert_eq!(error.to_string(), original.to_string());
        assert_eq!(error.exit_code(), original.exit_code());
        assert_eq!(error.use_stderr(), original.use_stderr());
    }

    #[test]
    fn help_requests_notify_before_clap_exits() {
        for args in [
            vec!["skm", "help"],
            vec!["skm", "--help"],
            vec!["skm", "self", "upgrade", "--help"],
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
        for args in [
            vec!["skm", "self", "version"],
            vec!["skm", "--version"],
            vec!["skm", "--invalid"],
            vec!["skm", "self"],
            vec!["skm", "add"],
        ] {
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
    fn retired_commands_are_absent_and_bundle_add_remains_available() {
        for args in [
            vec!["skm", "bundle", "add", "acme/starter"],
            vec!["skm", "dev", "mode", "on"],
            vec!["skm", "init-config"],
        ] {
            assert!(Cli::try_parse_from(args).is_err());
        }
        let parsed = Cli::try_parse_from([
            "skm",
            "add",
            "acme/starter",
            "--kind",
            "bundle",
            "--registry",
            "local",
            "--dry-run",
        ])
        .unwrap();
        assert!(matches!(
            parsed.command,
            Commands::Add { dry_run: true, .. }
        ));
        let add_help = Cli::try_parse_from(["skm", "add", "--help"])
            .err()
            .expect("help exits through Clap")
            .to_string();
        for flag in ["--kind", "--registry", "--dry-run", "--json", "--yes"] {
            assert!(add_help.contains(flag));
        }
        assert!(!add_help.contains("--source"));
        assert!(Cli::try_parse_from(["skm", "add", "acme/alpha", "--source", "local"]).is_ok());
        assert!(Cli::try_parse_from(["skm", "registry", "set-default", "local"]).is_ok());
        assert!(Cli::try_parse_from(["skm", "registry", "default", "local"]).is_ok());
        assert!(
            Cli::try_parse_from(["skm", "config", "get", "name", "--global", "--project"]).is_err()
        );
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
        ])
        .unwrap();
        let config = SkillsConfig::load_from_file(project.path()).unwrap();
        assert_eq!(config.toolkit.unwrap().version, "0.2.0");
        assert_eq!(config.bundles, ["core"]);
        assert_eq!(config.profiles, ["reviewer"]);
        assert!(config.metadata.is_empty());
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
        assert!(!help.contains("--interactive"));
        assert!(!help.contains("--advanced"));
    }

    #[test]
    fn init_exposes_generic_toolkits_and_rejects_retired_domain_options() {
        use clap::CommandFactory;
        let mut command = Cli::command();
        let help = command
            .find_subcommand_mut("init")
            .unwrap()
            .render_long_help()
            .to_string();
        assert!(help.contains("--toolkit-manifest"));
        assert!(!help.to_lowercase().contains("workspace"));
        for option in [
            "--workspace-standard",
            "--workspace-source",
            "--workspace-revision",
            "--workspace-integrity",
        ] {
            assert!(
                Cli::try_parse_from(["skm", "init", "--non-interactive", option, "value"]).is_err()
            );
        }
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
            metadata: Default::default(),
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
            metadata: Default::default(),
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

        run(Commands::Add {
            name: "acme/starter".into(),
            source: Some("local".into()),
            kind: None,
            path: None,
            global: true,
            dry_run: true,
            json: false,
            yes: false,
        })
        .unwrap_err();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        run(add("acme/beta", true, false)).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        run(Commands::Add {
            name: "acme/beta".into(),
            source: Some("local".into()),
            kind: None,
            path: None,
            global: false,
            dry_run: false,
            json: true,
            yes: false,
        })
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!linker::resolve_registry_path("local").unwrap().exists());
        run(add("acme/starter", true, false)).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(!environment.project().join(".agents/skills/alpha").exists());
        assert!(run(add("acme/starter", false, false))
            .unwrap_err()
            .to_string()
            .contains("--yes"));
        run(add("acme/starter", false, true)).unwrap();
        let after_bundle = fs::read(&config_path).unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 1);
        assert!(config.skills[0].is_bundle());
        assert_eq!(config.skills[0].source.as_deref(), Some("local"));
        run(add("acme/starter", false, true)).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), after_bundle);
        assert!(!environment.project().join(".agents/skills/alpha").exists());

        assert!(run(add("acme/beta", false, false))
            .unwrap_err()
            .to_string()
            .contains("--yes"));
        run(add("acme/beta", false, true)).unwrap();
        let config = SkillsConfig::load_from_file(&config_path).unwrap();
        assert_eq!(config.skills.len(), 2);
        assert!(!config.skills[1].is_bundle());
        assert!(!environment.project().join(".agents/skills/beta").exists());
        run(add("acme/beta", true, false)).unwrap();
    }

    #[test]
    #[serial]
    fn bundle_validation_and_uncached_preview_leave_no_writes() {
        let environment = Environment::new();
        let registry = environment.registry();
        let namespace = registry.join("skills/acme");
        fs::create_dir_all(&namespace).unwrap();
        let manifest = namespace.join("manifest.yaml");
        let config_path = environment.project().join("skills.yaml");
        let mut config = SkillsConfig {
            name: "validation".into(),
            version: None,
            registries: Some(HashMap::from([(
                "local".into(),
                registry.to_string_lossy().into_owned(),
            )])),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        };
        config.save_to_file(&config_path).unwrap();
        let before = fs::read(&config_path).unwrap();
        for contents in [
            "schema_version: 1\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n",
            "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha, alpha]}}\n",
            "schema_version: 2\nnamespace: acme\nskm_adapter_compatibility: 99.x\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n",
        ] {
            fs::write(&manifest, contents).unwrap();
            assert!(add_registry_item(&config_path, &environment.project(), "acme/starter", Some("local".into()), Some(AddKind::Bundle), None, false, true, false, false).is_err());
            assert_eq!(fs::read(&config_path).unwrap(), before);
        }
        config.registries = Some(HashMap::from([(
            "remote".into(),
            "https://example.invalid/repo.git".into(),
        )]));
        config.save_to_file(&config_path).unwrap();
        let before_remote = fs::read(&config_path).unwrap();
        let error = add_registry_item(
            &config_path,
            &environment.project(),
            "acme/starter",
            Some("remote".into()),
            None,
            None,
            false,
            true,
            false,
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("skm cache refresh remote"));
        assert_eq!(fs::read(&config_path).unwrap(), before_remote);
        assert!(!linker::resolve_registry_path("remote").unwrap().exists());
        assert!(!environment.project().join(".agents").exists());
    }

    #[test]
    #[serial]
    fn plain_local_registry_add_then_install_uses_same_source() {
        let environment = Environment::new();
        let project = environment.project();
        let registry = project.join("registry");
        let skill = registry.join("skills/acme/alpha/v1.0.0");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: alpha\nmetadata:\n  skm-version: 1.0.0\n---\n# Alpha\n",
        )
        .unwrap();
        std::os::unix::fs::symlink("v1.0.0", skill.parent().unwrap().join("latest")).unwrap();
        fs::write(registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n").unwrap();
        let registries = HashMap::from([("local".into(), "registry".into())]);
        BaseConfig {
            default_registry: "local".into(),
            registries: registries.clone(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let config_path = project.join("skills.yaml");
        fs::write(&config_path, "name: fixture\nregistries: {local: registry}\nagents: [codex]\nskills:\n  - name: old\n    path: old\n    publisher_hint: keep-me\n").unwrap();
        let old = project.join("old");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("SKILL.md"), "---\nname: old\n---\n# Old\n").unwrap();
        add_registry_item(
            &config_path,
            &project,
            "acme/starter",
            Some("local".into()),
            None,
            None,
            false,
            false,
            false,
            true,
        )
        .unwrap();
        let doc: serde_yaml::Value =
            serde_yaml::from_slice(&fs::read(&config_path).unwrap()).unwrap();
        assert_eq!(doc["skills"][0]["publisher_hint"].as_str(), Some("keep-me"));
        assert!(!project.join(".agents/skills/alpha").exists());
        run(Commands::Install {
            global: false,
            dry_run: false,
            json: false,
            yes: true,
        })
        .unwrap();
        assert!(project.join(".agents/skills/alpha").is_symlink());
        assert_eq!(
            fs::canonicalize(linker::resolve_registry_path("local").unwrap()).unwrap(),
            fs::canonicalize(registry).unwrap()
        );
    }

    #[test]
    #[serial]
    fn relative_git_registry_paths_survive_repeated_add_and_install() {
        let environment = Environment::new();
        let project = environment.project();
        let registry = environment.root.path().join("registry with spaces");
        let directory = registry.join("skills/acme/alpha/v1.0.0");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("SKILL.md"),
            "---\nname: alpha\nmetadata:\n  skm-version: 1.0.0\n---\n# Alpha\n",
        )
        .unwrap();
        std::os::unix::fs::symlink("v1.0.0", directory.parent().unwrap().join("latest")).unwrap();
        fs::write(registry.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n").unwrap();
        fs::create_dir_all(environment.root.path().join("a")).unwrap();
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
        let registries = HashMap::from([
            ("local".into(), "../registry with spaces".into()),
            ("alternate".into(), "../a/../registry with spaces".into()),
        ]);
        BaseConfig {
            default_registry: "local".into(),
            registries: registries.clone(),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        let config_path = project.join("skills.yaml");
        SkillsConfig {
            name: "relative".into(),
            version: None,
            registries: Some(registries),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        }
        .save_to_file(&config_path)
        .unwrap();
        let add = || {
            add_registry_item(
                &config_path,
                &project,
                "acme/starter",
                Some("local".into()),
                None,
                None,
                false,
                false,
                false,
                true,
            )
        };
        add().unwrap();
        let install = || {
            run(Commands::Install {
                global: false,
                dry_run: false,
                json: false,
                yes: true,
            })
        };
        install().unwrap();
        run(Commands::Check { global: false }).unwrap();
        let before = fs::read(&config_path).unwrap();
        add().unwrap();
        install().unwrap();
        run(Commands::Check { global: false }).unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        assert!(project.join(".agents/skills/alpha").is_symlink());
        fs::create_dir_all(project.join("different/skills")).unwrap();
        let mut changed: serde_yaml::Value = serde_yaml::from_slice(&before).unwrap();
        changed["registries"]["local"] = serde_yaml::Value::String("different".into());
        fs::write(&config_path, serde_yaml::to_string(&changed).unwrap()).unwrap();
        assert!(run(Commands::Check { global: false })
            .unwrap_err()
            .to_string()
            .contains("different or missing origin"));
        assert!(project.join(".agents/skills/alpha").is_symlink());
        fs::write(&config_path, &before).unwrap();
        assert!(linker::registry_cache_matches(
            &linker::resolve_registry_path("alternate").unwrap(),
            "../a/../registry with spaces",
            &project
        ));
    }

    #[test]
    #[serial]
    fn conflicting_cached_origins_are_rejected_and_cached_preview_needs_no_tmpdir() {
        let environment = Environment::new();
        let project = environment.project();
        let a = environment.registry();
        let b = project.join("other");
        for root in [&a, &b] {
            let skill = root.join("skills/acme/alpha/1.0.0");
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "---\nname: alpha\n---\n# Alpha\n").unwrap();
            fs::write(root.join("skills/acme/manifest.yaml"), "schema_version: 2\nnamespace: acme\npackages: {alpha: 1.0.0}\nbundles: {starter: {packages: [alpha]}}\n").unwrap();
            git(root, &["init", "--quiet"]);
            git(root, &["add", "."]);
            git(
                root,
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
        }
        let cache = linker::resolve_registry_path("local").unwrap();
        fs::create_dir_all(cache.parent().unwrap()).unwrap();
        let cloned = Command::new("git")
            .args(["clone", "--quiet"])
            .arg(&a)
            .arg(&cache)
            .status()
            .unwrap();
        assert!(cloned.success());
        let config_path = project.join("skills.yaml");
        let mut config = SkillsConfig {
            name: "origin".into(),
            version: None,
            registries: Some(HashMap::from([("local".into(), "other".into())])),
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        };
        BaseConfig {
            default_registry: "local".into(),
            registries: HashMap::from([("local".into(), a.to_string_lossy().into_owned())]),
            check_for_updates: false,
        }
        .save()
        .unwrap();
        config.save_to_file(&config_path).unwrap();
        let before = fs::read(&config_path).unwrap();
        let add = || {
            add_registry_item(
                &config_path,
                &project,
                "acme/starter",
                Some("local".into()),
                None,
                None,
                false,
                false,
                true,
                false,
            )
        };
        assert!(add()
            .unwrap_err()
            .to_string()
            .contains("different or missing origin"));
        config
            .registries
            .as_mut()
            .unwrap()
            .insert("local".into(), format!("file://{}", b.display()));
        config.save_to_file(&config_path).unwrap();
        assert!(add()
            .unwrap_err()
            .to_string()
            .contains("different or missing origin"));
        let origin = format!("file://{}", a.display());
        git(&cache, &["remote", "set-url", "origin", &origin]);
        config
            .registries
            .as_mut()
            .unwrap()
            .insert("local".into(), origin);
        config.save_to_file(&config_path).unwrap();
        let preview_before = fs::read(&config_path).unwrap();
        let tmpdir = env::var_os("TMPDIR");
        env::set_var("TMPDIR", project.join("missing-temp-dir"));
        let result = add();
        if let Some(value) = tmpdir {
            env::set_var("TMPDIR", value);
        } else {
            env::remove_var("TMPDIR");
        }
        result.unwrap();
        assert!(!project.join("missing-temp-dir").exists());
        assert_ne!(preview_before, before);
        assert_eq!(fs::read(&config_path).unwrap(), preview_before);
        assert!(!project.join(".agents").exists());
        config.skills.push(SkillSpec {
            name: "acme/alpha".into(),
            version: Some("latest".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        });
        config
            .registries
            .as_mut()
            .unwrap()
            .insert("local".into(), "other".into());
        assert!(validate_effective_registry_caches(&config, &project)
            .unwrap_err()
            .to_string()
            .contains("different or missing origin"));
    }

    #[test]
    fn skill_json_preview_contains_planned_entry() {
        let entry = SkillSpec {
            name: "acme/alpha".into(),
            version: Some("latest".into()),
            source: Some("local".into()),
            path: None,
            kind: None,
        };
        let result = format_add_preview(&entry, false, false).unwrap();
        let json: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(json["entry"]["name"], "acme/alpha");
        assert_eq!(json["entry"]["source"], "local");
        assert_eq!(json["entry"]["version"], "latest");
        assert_eq!(json["scope"], "project");
        assert_eq!(json["already_present"], false);
    }

    #[test]
    #[serial]
    fn single_skill_confirmation_precedes_project_writes() {
        let environment = Environment::new();
        let config_path = environment.project().join("skills.yaml");
        SkillsConfig {
            name: "confirmation".into(),
            version: None,
            registries: None,
            agents: vec!["codex".into()],
            skills: Vec::new(),
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
        }
        .save_to_file(&config_path)
        .unwrap();
        let requested = SkillSpec {
            name: "acme/alpha".into(),
            version: Some("latest".into()),
            source: None,
            path: Some("local-skill".into()),
            kind: None,
        };
        let before = fs::read(&config_path).unwrap();
        add_skill_with_confirmation(
            &config_path,
            requested.clone(),
            false,
            false,
            false,
            false,
            |question| {
                assert_eq!(question, "Add this entry to skills.yaml?");
                Ok(false)
            },
        )
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        add_skill_with_confirmation(
            &config_path,
            requested.clone(),
            false,
            true,
            false,
            false,
            |_| panic!("preview prompted"),
        )
        .unwrap();
        add_skill_with_confirmation(
            &config_path,
            requested.clone(),
            false,
            false,
            true,
            false,
            |_| panic!("JSON prompted"),
        )
        .unwrap();
        assert_eq!(fs::read(&config_path).unwrap(), before);
        add_skill_with_confirmation(&config_path, requested, false, false, false, false, |_| {
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
        assert!(!environment.project().join(".agents").exists());
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
            metadata: Default::default(),
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
                kind: None,
            }],
            toolkit: None,
            bundles: Vec::new(),
            profiles: Vec::new(),
            metadata: Default::default(),
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
