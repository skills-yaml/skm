use std::io::{self, Write};
use std::collections::HashMap;
use crate::config::{SkillSpec, SkillsConfig};

/// Available agents categorized by type
const AVAILABLE_AGENTS: &[&str] = &[
    "claude",
    "codex", 
    "cursor",
    "copilot",
    "grok",
    "hermes",
];

/// Coding-specific agents
pub const CODING_AGENTS: &[&str] = &[
    "codex",
    "cursor",
    "copilot",
    "claude",
];

/// Available skills in the default registry
pub const AVAILABLE_SKILLS: &[&str] = &[
    "software-development/spec",
    "software-development/symphony-spec-writing",
    "system/devops-manager",
    "system/cloud-architecture",
    "data/data-analysis",
    "security/security-audit",
    "writing/technical-writing",
    "writing/documentation",
];

/// Skill categories for better organization
pub const SKILL_CATEGORIES: &[(&str, &[&str])] = &[
    ("Software Development", &["software-development/spec", "software-development/symphony-spec-writing"]),
    ("System & DevOps", &["system/devops-manager", "system/cloud-architecture"]),
    ("Data & Analytics", &["data/data-analysis"]),
    ("Security", &["security/security-audit"]),
    ("Writing", &["writing/technical-writing", "writing/documentation"]),
];

/// Represents a selectable item in the wizard
#[derive(Debug, Clone)]
pub struct SelectableItem {
    pub index: usize,
    pub name: String,
    pub description: Option<String>,
    pub selected: bool,
}

impl SelectableItem {
    pub fn with_description(index: usize, name: &str, description: &str) -> Self {
        Self {
            index,
            name: name.to_string(),
            description: Some(description.to_string()),
            selected: false,
        }
    }
}

/// Display a multi-select menu and return selected indices
pub fn multi_select_menu(
    title: &str,
    items: &mut [SelectableItem],
    allow_none: bool,
    default_select_all: bool,
) -> io::Result<Vec<usize>> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    println!("\n{}== {} =={}", "=".repeat(20), title, "=".repeat(20));
    
    // Display all items
    for item in items.iter_mut() {
        if default_select_all {
            item.selected = true;
        }
        let marker = if item.selected { "[x]" } else { "[ ]" };
        let desc = item.description.as_deref().unwrap_or("");
        println!("{} [{}] {} - {}", marker, item.index, item.name, desc);
    }

    println!("\n{}Options:", " ".repeat(4));
    println!("  [a] Select all");
    println!("  [n] Select none");
    println!("  [t] Toggle selection");
    println!("  [1-9] Toggle specific item");
    println!("  [d] Done");
    
    if allow_none {
        println!("  [q] Continue with no selection");
    }

    loop {
        print!("\n> ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "a" | "all" => {
                for item in items.iter_mut() {
                    item.selected = true;
                }
                display_selection_state(items);
            }
            "n" | "none" => {
                for item in items.iter_mut() {
                    item.selected = false;
                }
                display_selection_state(items);
            }
            "t" | "toggle" => {
                // Toggle all
                let all_selected = items.iter().all(|i| i.selected);
                for item in items.iter_mut() {
                    item.selected = !all_selected;
                }
                display_selection_state(items);
            }
            "d" | "done" => {
                let selected_indices: Vec<usize> = items
                    .iter()
                    .filter(|i| i.selected)
                    .map(|i| i.index)
                    .collect();

                if !allow_none && selected_indices.is_empty() {
                    println!("At least one item must be selected.");
                    continue;
                }
                return Ok(selected_indices);
            }
            "q" | "quit" | "" => {
                if allow_none {
                    return Ok(Vec::new());
                } else {
                    println!("At least one item must be selected. Use 'q' only if selection is optional.");
                }
            }
            _ => {
                // Try to parse as number
                if let Ok(num) = input.parse::<usize>() {
                    if let Some(item) = items.iter_mut().find(|i| i.index == num) {
                        item.selected = !item.selected;
                        display_selection_state(items);
                    } else {
                        println!("Invalid item number. Please enter a valid number.");
                    }
                } else {
                    println!("Invalid input. Please enter a number, 'a' for all, 'n' for none, 't' to toggle, or 'd' when done.");
                }
            }
        }
    }
}

/// Display the current selection state
fn display_selection_state(items: &[SelectableItem]) {
    println!("\nCurrent selection:");
    let selected: Vec<&str> = items
        .iter()
        .filter(|i| i.selected)
        .map(|i| i.name.as_str())
        .collect();
    
    if selected.is_empty() {
        println!("  (none)");
    } else {
        for name in &selected {
            println!("  - {}", name);
        }
    }
}

/// Simple yes/no confirmation prompt
pub fn confirm_prompt(message: &str, default: bool) -> io::Result<bool> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    let default_str = if default { "[Y/n]" } else { "[y/N]" };
    print!("{} {} ", message, default_str);
    stdout.flush()?;

    let mut input = String::new();
    stdin.read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    match input.as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        "" => Ok(default),
        _ => Ok(default),
    }
}

/// Simple text input with default
pub fn text_input(prompt: &str, default: &str) -> io::Result<String> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    print!("{} [{}]: ", prompt, default);
    stdout.flush()?;

    let mut input = String::new();
    stdin.read_line(&mut input)?;
    let input = input.trim();

    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

/// Get project name from user
pub fn get_project_name(default_name: &str) -> io::Result<String> {
    text_input("Project name", default_name)
}

/// Get agent selection from user
pub fn select_agents() -> io::Result<Vec<String>> {
    let mut agents: Vec<SelectableItem> = AVAILABLE_AGENTS
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            SelectableItem::with_description(
                i + 1,
                name,
                get_agent_description(name)
            )
        })
        .collect();

    let selected_indices = multi_select_menu(
        "Select Agents",
        &mut agents,
        false,
        true, // Default: select all
    )?;

    Ok(selected_indices
        .into_iter()
        .map(|idx| AVAILABLE_AGENTS[idx - 1].to_string())
        .collect())
}

/// Get coding agent selection from user
pub fn select_coding_agents() -> io::Result<Vec<String>> {
    let mut agents: Vec<SelectableItem> = CODING_AGENTS
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            SelectableItem::with_description(
                i + 1,
                name,
                get_coding_agent_description(name)
            )
        })
        .collect();

    let selected_indices = multi_select_menu(
        "Select Coding Agents",
        &mut agents,
        true, // Allow none
        false, // Default: don't select all
    )?;

    Ok(selected_indices
        .into_iter()
        .map(|idx| CODING_AGENTS[idx - 1].to_string())
        .collect())
}

/// Enhanced skill selection with search and filtering
pub fn select_skills_enhanced() -> io::Result<Vec<SkillSpec>> {
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    println!("\n{}== Select Skills =={}", "=".repeat(20), "=".repeat(20));
    
    // Show categories
    for (i, (category, skills)) in SKILL_CATEGORIES.iter().enumerate() {
        println!("\n[{}] {}", i + 1, category);
        for (j, skill) in skills.iter().enumerate() {
            println!("    [{}] {}", j + 1, skill);
        }
    }

    println!("\n{}Options:", " ".repeat(4));
    println!("  [c <num>] Select category by number");
    println!("  [s <num>] Select specific skill by number");
    println!("  [a] Select all");
    println!("  [n] Select none");
    println!("  [d] Done");
    println!("  [q] Continue with no selection");

    let mut selected_skills: Vec<SkillSpec> = Vec::new();
    let all_skills: Vec<&str> = SKILL_CATEGORIES
        .iter()
        .flat_map(|(_, skills)| skills.iter())
        .copied()
        .collect();

    loop {
        print!("\n> ");
        stdout.flush()?;

        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input.is_empty() || input == "d" || input == "done" {
            break;
        }

        if input == "q" || input == "quit" {
            return Ok(Vec::new());
        }

        if input == "a" || input == "all" {
            selected_skills = all_skills
                .iter()
                .map(|&name| SkillSpec {
                    name: name.to_string(),
                    version: Some("latest".to_string()),
                    source: Some("default".to_string()),
                    path: None,
                })
                .collect();
            println!("All skills selected. Current count: {}", selected_skills.len());
            continue;
        }

        if input == "n" || input == "none" {
            selected_skills.clear();
            println!("All skills deselected.");
            continue;
        }

        // Handle category selection: c <num>
        if input.starts_with("c ") || input.starts_with("category ") {
            if let Ok(cat_num) = input.trim_start_matches("c ").parse::<usize>() {
                if cat_num > 0 && cat_num <= SKILL_CATEGORIES.len() {
                    let (_, category_skills) = SKILL_CATEGORIES[cat_num - 1];
                    for &skill_name in category_skills {
                        if !selected_skills.iter().any(|s| s.name == skill_name) {
                            selected_skills.push(SkillSpec {
                                name: skill_name.to_string(),
                                version: Some("latest".to_string()),
                                source: Some("default".to_string()),
                                path: None,
                            });
                        }
                    }
                    println!("Added {} skills from category. Total: {}", category_skills.len(), selected_skills.len());
                    continue;
                }
            }
            println!("Invalid category number.");
            continue;
        }

        // Handle skill selection: s <num> or just <num>
        let skill_num_str = input.trim_start_matches("s ");
        if let Ok(skill_num) = skill_num_str.parse::<usize>() {
            if skill_num > 0 && skill_num <= all_skills.len() {
                let skill_name = all_skills[skill_num - 1];
                if !selected_skills.iter().any(|s| s.name == skill_name) {
                    selected_skills.push(SkillSpec {
                        name: skill_name.to_string(),
                        version: Some("latest".to_string()),
                        source: Some("default".to_string()),
                        path: None,
                    });
                    println!("Added: {}. Total: {}", skill_name, selected_skills.len());
                } else {
                    println!("Skill '{}' already selected.", skill_name);
                }
                continue;
            }
        }

        println!("Invalid input. Use 'c <num>' for category, 's <num>' for skill, or see options above.");
    }

    Ok(selected_skills)
}

/// Get agent description
fn get_agent_description(name: &str) -> &'static str {
    match name {
        "claude" => "Anthropic Claude - Advanced AI assistant",
        "codex" => "GitHub Codex - AI coding assistant",
        "cursor" => "Cursor - AI-powered code editor",
        "copilot" => "GitHub Copilot - AI pair programmer",
        "grok" => "xAI Grok - AI with real-time knowledge",
        "hermes" => "Hermes - Local LLM interface",
        _ => "Unknown agent",
    }
}

/// Get coding agent description
fn get_coding_agent_description(name: &str) -> &'static str {
    match name {
        "codex" => "GitHub Codex - Specialized for code generation",
        "cursor" => "Cursor - AI-powered code editor with deep integration",
        "copilot" => "GitHub Copilot - AI pair programmer for coding tasks",
        "claude" => "Claude Code - Advanced coding assistant by Anthropic",
        _ => "Unknown coding agent",
    }
}

/// Get skill description
fn get_skill_description(name: &str) -> &'static str {
    match name {
        "software-development/spec" => "Create technical specifications",
        "software-development/symphony-spec-writing" => "Write Symphony specifications",
        "system/devops-manager" => "DevOps and infrastructure management",
        "system/cloud-architecture" => "Cloud architecture design",
        "data/data-analysis" => "Data analysis and visualization",
        "security/security-audit" => "Security auditing and analysis",
        "writing/technical-writing" => "Technical documentation writing",
        "writing/documentation" => "General documentation",
        _ => "Unknown skill",
    }
}

/// Main interactive wizard for init
pub fn run_wizard(
    name: Option<String>,
    global: bool,
) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    println!("\n{}== SKM Configuration Wizard =={}", "=".repeat(20), "=".repeat(20));
    println!("This wizard will help you set up your skills configuration.");

    // Step 1: Project name
    let current_dir_name = std::env::current_dir()?
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("my-project")
        .to_string();
    
    let project_name = if let Some(ref n) = name {
        n.clone()
    } else {
        get_project_name(&current_dir_name)?
    };
    
    println!("\nProject name: {}", project_name);

    // Step 2: Select agents
    println!("\n--- Step 1: Select Agents ---");
    let agents = select_agents()?;
    println!("Selected agents: {:?}", agents);

    // Step 3: Select coding agents (subset of agents, specialized for coding)
    println!("\n--- Step 2: Select Coding Agents (Optional) ---");
    println!("Coding agents are specialized for development tasks.");
    let coding_agents = select_coding_agents()?;
    println!("Selected coding agents: {:?}", coding_agents);

    // Step 4: Select skills
    println!("\n--- Step 3: Select Skills ---");
    let skills = select_skills_enhanced()?;
    println!("Selected skills: {:?}", skills.iter().map(|s| &s.name).collect::<Vec<_>>());

    // Step 5: Configure registries
    println!("\n--- Step 4: Configure Registries ---");
    let use_default_registry = confirm_prompt("Use default skills registry?", true)?;
    
    let mut registries = HashMap::new();
    if use_default_registry {
        registries.insert(
            "default".to_string(),
            "git@github.com:skills-yaml/skills-registry.git".to_string(),
        );
    }

    // Step 6: Version
    let version = text_input("Project version", "0.1.0")?;

    // Combine agents and coding agents (deduplicated)
    let mut all_agents = agents.clone();
    for ca in &coding_agents {
        if !all_agents.contains(ca) {
            all_agents.push(ca.clone());
        }
    }

    println!("\n{}Configuration Summary:{}", "=".repeat(20), "=".repeat(20));
    println!("  Project name: {}", project_name);
    println!("  Version: {}", version);
    println!("  Agents: {:?}", all_agents);
    println!("  Coding Agents: {:?}", coding_agents);
    println!("  Skills: {:?}", skills.iter().map(|s| &s.name).collect::<Vec<_>>());
    println!("  Registries: {:?}", registries);

    let confirmed = confirm_prompt("\nConfirm configuration?", true)?;
    
    if !confirmed {
        println!("Configuration cancelled. Starting over...");
        return run_wizard(name.clone(), global);
    }

    Ok(SkillsConfig {
        name: project_name,
        version: Some(version),
        registries: Some(registries),
        agents: all_agents,
        skills,
    })
}

/// Run a simplified, streamlined interactive init
pub fn run_streamlined_wizard(
    name: Option<String>,
    _global: bool,
) -> Result<SkillsConfig, Box<dyn std::error::Error>> {
    println!("\nSKM Interactive Setup");
    println!("{}", "-".repeat(40));

    // Step 1: Project name
    let current_dir_name = std::env::current_dir()?
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("my-project")
        .to_string();
    
    let project_name = if let Some(n) = name {
        n
    } else {
        print!("Project name [{}]: ", current_dir_name);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        if input.is_empty() {
            current_dir_name
        } else {
            input.to_string()
        }
    };

    // Step 2: Agents with nice display
    println!("\nAvailable Agents:");
    for (i, agent) in AVAILABLE_AGENTS.iter().enumerate() {
        println!("  [{}] {} - {}", i + 1, agent, get_agent_description(agent));
    }
    
    print!("\nSelect agents (comma-separated numbers, or 'a' for all) [all]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    let selected_agents: Vec<String> = if input == "a" || input.is_empty() {
        AVAILABLE_AGENTS.iter().map(|&s| s.to_string()).collect()
    } else {
        input
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&i| i > 0 && i <= AVAILABLE_AGENTS.len())
            .map(|i| AVAILABLE_AGENTS[i - 1].to_string())
            .collect()
    };

    println!("Selected agents: {:?}", selected_agents);

    // Step 3: Coding agents
    println!("\nCoding Agents (specialized for development):");
    for (i, agent) in CODING_AGENTS.iter().enumerate() {
        println!("  [{}] {} - {}", i + 1, agent, get_coding_agent_description(agent));
    }
    
    print!("\nSelect coding agents (comma-separated numbers, or 'a' for all) [none]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    let coding_agents: Vec<String> = if input == "a" {
        CODING_AGENTS.iter().map(|&s| s.to_string()).collect()
    } else if input.is_empty() {
        Vec::new()
    } else {
        input
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&i| i > 0 && i <= CODING_AGENTS.len())
            .map(|i| CODING_AGENTS[i - 1].to_string())
            .collect()
    };

    println!("Selected coding agents: {:?}", coding_agents);

    // Step 4: Skills
    println!("\nAvailable Skills:");
    for (i, skill) in AVAILABLE_SKILLS.iter().enumerate() {
        println!("  [{}] {} - {}", i + 1, skill, get_skill_description(skill));
    }
    
    print!("\nSelect skills (comma-separated numbers, or 'a' for all) [none]: ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    let selected_skills: Vec<SkillSpec> = if input == "a" {
        AVAILABLE_SKILLS
            .iter()
            .map(|&name| SkillSpec {
                name: name.to_string(),
                version: Some("latest".to_string()),
                source: Some("default".to_string()),
                path: None,
            })
            .collect()
    } else if input.is_empty() {
        Vec::new()
    } else {
        input
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .filter(|&i| i > 0 && i <= AVAILABLE_SKILLS.len())
            .map(|i| SkillSpec {
                name: AVAILABLE_SKILLS[i - 1].to_string(),
                version: Some("latest".to_string()),
                source: Some("default".to_string()),
                path: None,
            })
            .collect()
    };

    println!("Selected skills: {:?}", selected_skills.iter().map(|s| &s.name).collect::<Vec<_>>());

    // Combine agents
    let mut all_agents = selected_agents.clone();
    for ca in &coding_agents {
        if !all_agents.contains(ca) {
            all_agents.push(ca.clone());
        }
    }

    // Build config
    let mut registries = HashMap::new();
    registries.insert(
        "default".to_string(),
        "git@github.com:skills-yaml/skills-registry.git".to_string(),
    );

    Ok(SkillsConfig {
        name: project_name,
        version: Some("0.1.0".to_string()),
        registries: Some(registries),
        agents: all_agents,
        skills: selected_skills,
    })
}
