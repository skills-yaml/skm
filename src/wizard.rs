mod draft;
mod prompt;

pub use draft::Document;
pub use prompt::run_wizard;

const KNOWN_AGENTS: &[&str] = &["claude", "codex", "cursor", "copilot", "grok", "hermes"];

/// Returns a list of agents that are actually available in the user's environment
pub fn detect_available_agents() -> Vec<String> {
    KNOWN_AGENTS
        .iter()
        .filter(|&agent| is_agent_available(agent))
        .map(|&s| s.to_string())
        .collect()
}

/// Check if a specific agent is available by checking for its directory
fn is_agent_available(agent: &str) -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };

    let agent_dir = match agent {
        "claude" => home.join(".claude"),
        "codex" => home.join(".codex"),
        "cursor" => home.join(".cursor"),
        "copilot" => {
            // Copilot can be in .github or .vscode/extensions
            if home.join(".github").exists() || home.join(".vscode").exists() {
                return true;
            }
            return false;
        }
        "grok" => home.join(".grok"),
        "hermes" => home.join(".hermes"),
        _ => return false,
    };

    agent_dir.exists()
}
