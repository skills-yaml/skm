use super::draft::{self, Document, Result, Target};
use crate::config::{SkillSpec, SkillsConfig};
use crate::search::{self, Discovery, Entry};
use serde_yaml::Value;
use std::collections::BTreeSet;
use std::io::{self, BufRead, IsTerminal, Write};

enum Answer {
    Text(String),
    Cancel,
}

pub fn run_wizard(document: &mut Document, global: bool) -> Result<bool> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err("Interactive init needs a terminal on stdin and stderr; use 'skm init --non-interactive' for scripts".into());
    }
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stderr = io::stderr();
    let mut output = stderr.lock();
    run_prompts(document, global, &mut input, &mut output)
}

fn run_prompts<R: BufRead, W: Write>(
    document: &mut Document,
    global: bool,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    writeln!(output, "SKM init")?;
    writeln!(
        output,
        "Configure {}. Press Enter to keep a value; enter :q to cancel.",
        document.path.display()
    )?;

    loop {
        if !edit_project(document, input, output)?
            || !edit_agents(document, input, output)?
            || !edit_registries(document, input, output)?
            || !edit_skills(document, input, output)?
            || !edit_workspace(document, input, output)?
        {
            return Ok(false);
        }

        if let Err(error) = document.validate(global) {
            writeln!(output, "\nCannot save: {error}")?;
            let Some(retry) = confirm(input, output, "Review the prompts again?", true)? else {
                return Ok(false);
            };
            if retry {
                continue;
            }
            return Ok(false);
        }

        writeln!(output, "\n== Review ==")?;
        writeln!(output, "{}", document.preview()?)?;
        let Some(save) = confirm(input, output, "Save skills.yaml?", true)? else {
            return Ok(false);
        };
        if !save {
            return Ok(false);
        }
        document.save(global)?;
        return Ok(true);
    }
}

fn edit_project<R: BufRead, W: Write>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    section(output, "Project")?;
    if !edit_target(
        document,
        input,
        output,
        Target::Field("name"),
        "Project name",
        false,
        |value| {
            if value.trim().is_empty() {
                Err("Project name cannot be empty".into())
            } else {
                Ok(())
            }
        },
    )? {
        return Ok(false);
    }
    edit_target(
        document,
        input,
        output,
        Target::Field("version"),
        "Project version",
        true,
        |_| Ok(()),
    )
}

fn edit_agents<R: BufRead, W: Write>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    section(output, "Agents")?;
    let mut choices: Vec<String> = super::KNOWN_AGENTS
        .iter()
        .map(|name| (*name).into())
        .collect();
    if let Some(existing) = document.value["agents"].as_sequence() {
        for name in existing.iter().filter_map(Value::as_str) {
            if !choices.iter().any(|choice| choice == name) {
                choices.push(name.to_owned());
            }
        }
    }
    let selected: Vec<String> = document.value["agents"]
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    for (index, name) in choices.iter().enumerate() {
        let marker = if selected.contains(name) { "x" } else { " " };
        writeln!(output, "  {}. [{marker}] {name}", index + 1)?;
    }
    loop {
        writeln!(
            output,
            "Enter numbers or names separated by commas, 'all', or 'none'."
        )?;
        match ask(input, output, "Agents [keep]: ")? {
            Answer::Cancel => return Ok(false),
            Answer::Text(value) if value.is_empty() => return Ok(true),
            Answer::Text(value) if value.eq_ignore_ascii_case("all") => {
                document.value["agents"] = Value::Sequence(
                    super::KNOWN_AGENTS
                        .iter()
                        .map(|name| Value::String((*name).into()))
                        .collect(),
                );
                return Ok(true);
            }
            Answer::Text(value) if value.eq_ignore_ascii_case("none") => {
                document.value["agents"] = Value::Sequence(Vec::new());
                return Ok(true);
            }
            Answer::Text(value) => match parse_choices(&value, &choices) {
                Ok(names) => {
                    document.value["agents"] =
                        Value::Sequence(names.into_iter().map(Value::String).collect());
                    return Ok(true);
                }
                Err(error) => writeln!(output, "Invalid selection: {error}")?,
            },
        }
    }
}

fn edit_registries<R: BufRead, W: Write>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    section(output, "Registries")?;
    loop {
        let entries = registry_entries(&document.value);
        if entries.is_empty() {
            writeln!(output, "  (none)")?;
        } else {
            for (index, (name, location)) in entries.iter().enumerate() {
                writeln!(output, "  {}. {name} -> {location}", index + 1)?;
            }
        }
        match ask(
            input,
            output,
            "Registry action [Enter continue, a add, e N edit, d N remove]: ",
        )? {
            Answer::Cancel => return Ok(false),
            Answer::Text(command) if command.is_empty() => return Ok(true),
            Answer::Text(command) => {
                let Some((action, index)) = parse_action(&command) else {
                    writeln!(output, "Use a, e <number>, d <number>, or Enter.")?;
                    continue;
                };
                match action {
                    'a' if index.is_none() => {
                        let field = draft::add_entry(&mut document.value, 2);
                        if !edit_registry(document, field / 2, input, output)? {
                            return Ok(false);
                        }
                    }
                    'e' => {
                        let Some(index) = valid_index(index, entries.len()) else {
                            writeln!(output, "Choose an existing registry number.")?;
                            continue;
                        };
                        if !edit_registry(document, index, input, output)? {
                            return Ok(false);
                        }
                    }
                    'd' => {
                        let Some(index) = valid_index(index, entries.len()) else {
                            writeln!(output, "Choose an existing registry number.")?;
                            continue;
                        };
                        let Some(remove) = confirm(
                            input,
                            output,
                            &format!("Remove registry '{}' from the draft?", entries[index].0),
                            false,
                        )?
                        else {
                            return Ok(false);
                        };
                        if remove {
                            draft::remove_entry(
                                &mut document.value,
                                &Target::Registry(index, false),
                            );
                        }
                    }
                    _ => writeln!(output, "Use a, e <number>, d <number>, or Enter.")?,
                }
            }
        }
    }
}

fn edit_registry<R: BufRead, W: Write>(
    document: &mut Document,
    index: usize,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    if !edit_target(
        document,
        input,
        output,
        Target::Registry(index, true),
        "Registry name",
        false,
        |value| {
            if crate::linker::is_safe_registry_name(value) {
                Ok(())
            } else {
                Err("use letters, numbers, hyphens, or underscores".into())
            }
        },
    )? {
        return Ok(false);
    }
    edit_target(
        document,
        input,
        output,
        Target::Registry(index, false),
        "Registry URL or local path",
        false,
        |value| {
            if value.trim().is_empty() {
                Err("a registry location is required".into())
            } else {
                Ok(())
            }
        },
    )
}

fn edit_skills<R: BufRead, W: Write>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    section(output, "Skills")?;
    let mut discovery: Option<Discovery> = None;
    loop {
        let skills = skill_names(&document.value);
        if skills.is_empty() {
            writeln!(output, "  (none)")?;
        } else {
            for (index, description) in skills.iter().enumerate() {
                writeln!(output, "  {}. {description}", index + 1)?;
            }
        }
        match ask(
            input,
            output,
            "Skill action [Enter continue, s search, r refresh, a add, e N edit, d N remove]: ",
        )? {
            Answer::Cancel => return Ok(false),
            Answer::Text(command) if command.is_empty() => return Ok(true),
            Answer::Text(command) => {
                let Some((action, index)) = parse_action(&command) else {
                    writeln!(output, "Use s, r, a, e <number>, d <number>, or Enter.")?;
                    continue;
                };
                match action {
                    's' | 'r' if index.is_none() => {
                        if action == 'r' || discovery.is_none() {
                            writeln!(output, "Loading configured registries...")?;
                            let project = document
                                .path
                                .parent()
                                .unwrap_or_else(|| std::path::Path::new("."));
                            let config = match serde_yaml::from_value::<SkillsConfig>(
                                document.value.clone(),
                            ) {
                                Ok(config) => config,
                                Err(error) => {
                                    writeln!(output, "Cannot search this draft: {error}")?;
                                    continue;
                                }
                            };
                            match search::discover_with_refresh(
                                Some(&config),
                                project,
                                None,
                                action == 'r',
                            ) {
                                Ok(found) => discovery = Some(found),
                                Err(error) => {
                                    writeln!(output, "Could not load registries: {error}")?;
                                    continue;
                                }
                            }
                        }
                        if !search_skills(
                            document,
                            discovery.as_ref().expect("discovery was loaded"),
                            input,
                            output,
                        )? {
                            return Ok(false);
                        }
                    }
                    'a' if index.is_none() => {
                        let field = draft::add_entry(&mut document.value, 3);
                        if !edit_skill(document, field / 4, input, output)? {
                            return Ok(false);
                        }
                    }
                    'e' => {
                        let Some(index) = valid_index(index, skills.len()) else {
                            writeln!(output, "Choose an existing skill number.")?;
                            continue;
                        };
                        if !edit_skill(document, index, input, output)? {
                            return Ok(false);
                        }
                    }
                    'd' => {
                        let Some(index) = valid_index(index, skills.len()) else {
                            writeln!(output, "Choose an existing skill number.")?;
                            continue;
                        };
                        let name = document.value["skills"][index]["name"]
                            .as_str()
                            .unwrap_or("unnamed skill");
                        let Some(remove) = confirm(
                            input,
                            output,
                            &format!("Remove skill '{name}' from the draft?"),
                            false,
                        )?
                        else {
                            return Ok(false);
                        };
                        if remove {
                            draft::remove_entry(&mut document.value, &Target::Skill(index, "name"));
                        }
                    }
                    _ => writeln!(output, "Use s, r, a, e <number>, d <number>, or Enter.")?,
                }
            }
        }
    }
}

fn search_skills<R: BufRead, W: Write>(
    document: &mut Document,
    discovery: &Discovery,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    for warning in &discovery.warnings {
        writeln!(output, "Warning: {warning}")?;
    }
    let query = match ask(input, output, "Search query [Enter back]: ")? {
        Answer::Cancel => return Ok(false),
        Answer::Text(query) if query.is_empty() => return Ok(true),
        Answer::Text(query) => query,
    };
    let matches: Vec<_> = discovery
        .entries
        .iter()
        .filter(|entry| entry_matches(entry, &query))
        .collect();
    if matches.is_empty() {
        writeln!(output, "No matching skills.")?;
        return Ok(true);
    }
    const LIMIT: usize = 20;
    for (index, entry) in matches.iter().take(LIMIT).enumerate() {
        let marker = if entry_selected(entry, &document.value) {
            "x"
        } else {
            " "
        };
        writeln!(
            output,
            "  {}. [{marker}] {} — {} @ {}",
            index + 1,
            entry.name,
            entry.registry,
            entry.version
        )?;
        if let Some(description) = &entry.description {
            writeln!(output, "     {description}")?;
        }
    }
    if matches.len() > LIMIT {
        writeln!(
            output,
            "  Showing the first {LIMIT} of {} matches; refine the query for more.",
            matches.len()
        )?;
    }
    loop {
        match ask(
            input,
            output,
            "Toggle result numbers, comma-separated [Enter back]: ",
        )? {
            Answer::Cancel => return Ok(false),
            Answer::Text(value) if value.is_empty() => return Ok(true),
            Answer::Text(value) => match parse_numbers(&value, matches.len().min(LIMIT)) {
                Ok(indices) => {
                    for index in indices {
                        if let Err(error) = toggle_entry(matches[index], &mut document.value) {
                            writeln!(output, "Cannot select '{}': {error}", matches[index].name)?;
                        }
                    }
                    return Ok(true);
                }
                Err(error) => writeln!(output, "Invalid selection: {error}")?,
            },
        }
    }
}

fn entry_matches(entry: &Entry, query: &str) -> bool {
    let haystack = format!("{} {} {}", entry.name, entry.registry, entry.version).to_lowercase();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

fn entry_same_source(entry: &Entry, skill: &Value) -> bool {
    skill["name"].as_str() == Some(&entry.name)
        && skill["path"].is_null()
        && skill["source"].as_str().unwrap_or("default") == entry.registry
}

fn entry_selected(entry: &Entry, document: &Value) -> bool {
    document["skills"]
        .as_sequence()
        .is_some_and(|skills| skills.iter().any(|skill| entry_same_source(entry, skill)))
}

fn toggle_entry(entry: &Entry, document: &mut Value) -> std::result::Result<(), String> {
    if !document["skills"].is_sequence() {
        document["skills"] = Value::Sequence(Vec::new());
    }
    let skills = document["skills"].as_sequence_mut().unwrap();
    if let Some(index) = skills
        .iter()
        .position(|skill| skill["name"].as_str() == Some(&entry.name))
    {
        if !entry_same_source(entry, &skills[index]) {
            return Err(format!(
                "'{}' already has a different source or local path. Use e to edit it.",
                entry.name
            ));
        }
        skills.remove(index);
    } else {
        skills.push(
            serde_yaml::to_value(SkillSpec {
                name: entry.name.clone(),
                version: Some(entry.version.clone()),
                source: Some(entry.registry.clone()),
                path: None,
            })
            .map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}

fn edit_skill<R: BufRead, W: Write>(
    document: &mut Document,
    index: usize,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    if !edit_target(
        document,
        input,
        output,
        Target::Skill(index, "name"),
        "Skill name",
        false,
        |value| crate::linker::validate_skill_name(value).map_err(|error| error.to_string()),
    )? {
        return Ok(false);
    }
    if !edit_target(
        document,
        input,
        output,
        Target::Skill(index, "version"),
        "Version",
        true,
        |_| Ok(()),
    )? {
        return Ok(false);
    }
    if !edit_target(
        document,
        input,
        output,
        Target::Skill(index, "source"),
        "Registry source",
        true,
        |value| {
            if value.is_empty() || crate::linker::is_safe_registry_name(value) {
                Ok(())
            } else {
                Err("use a configured registry name".into())
            }
        },
    )? {
        return Ok(false);
    }
    edit_target(
        document,
        input,
        output,
        Target::Skill(index, "path"),
        "Local path",
        true,
        |_| Ok(()),
    )
}

fn edit_workspace<R: BufRead, W: Write>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
) -> Result<bool> {
    section(output, "Optional toolkit and workspace settings")?;
    let configured = !document.value["toolkit"].is_null()
        || !document.value["workspace"].is_null()
        || document.value["bundles"]
            .as_sequence()
            .is_some_and(|v| !v.is_empty())
        || document.value["profiles"]
            .as_sequence()
            .is_some_and(|v| !v.is_empty())
        || document.value["trusted_sources"]
            .as_sequence()
            .is_some_and(|v| !v.is_empty());
    writeln!(
        output,
        "Current optional configuration: {}.",
        if configured { "configured" } else { "none" }
    )?;
    let Some(edit) = confirm(input, output, "Edit these advanced settings?", false)? else {
        return Ok(false);
    };
    if !edit {
        return Ok(true);
    }
    writeln!(output, "Enter - to clear an optional value.")?;
    if !edit_target(
        document,
        input,
        output,
        Target::Nested("toolkit", "manifest"),
        "Toolkit manifest",
        true,
        |_| Ok(()),
    )? {
        return Ok(false);
    }
    if !document.value["toolkit"].is_null()
        && !edit_target(
            document,
            input,
            output,
            Target::Nested("toolkit", "version"),
            "Toolkit version",
            false,
            |_| Ok(()),
        )?
    {
        return Ok(false);
    }
    for (target, label) in [
        (Target::Field("bundles"), "Bundles (comma-separated)"),
        (Target::Field("profiles"), "Profiles (comma-separated)"),
    ] {
        if !edit_target(document, input, output, target, label, true, |_| Ok(()))? {
            return Ok(false);
        }
    }
    if !edit_target(
        document,
        input,
        output,
        Target::Nested("workspace", "standard"),
        "Workspace standard",
        true,
        |_| Ok(()),
    )? {
        return Ok(false);
    }
    if !document.value["workspace"].is_null() {
        for (key, label) in [
            ("source", "Workspace source"),
            ("revision", "Workspace revision"),
            ("integrity", "Workspace integrity"),
        ] {
            if !edit_target(
                document,
                input,
                output,
                Target::Nested("workspace", key),
                label,
                true,
                |_| Ok(()),
            )? {
                return Ok(false);
            }
        }
    }
    edit_target(
        document,
        input,
        output,
        Target::Field("trusted_sources"),
        "Trusted sources (comma-separated)",
        true,
        |_| Ok(()),
    )
}

fn edit_target<R, W, F>(
    document: &mut Document,
    input: &mut R,
    output: &mut W,
    target: Target,
    label: &str,
    optional: bool,
    validate: F,
) -> Result<bool>
where
    R: BufRead,
    W: Write,
    F: Fn(&str) -> std::result::Result<(), String>,
{
    loop {
        let current = target.read(&document.value);
        let suffix = if current.is_empty() {
            "(empty)".to_owned()
        } else {
            current.clone()
        };
        let prompt = if optional {
            format!("{label} [{suffix}; - clears]: ")
        } else {
            format!("{label} [{suffix}]: ")
        };
        let value = match ask(input, output, &prompt)? {
            Answer::Cancel => return Ok(false),
            Answer::Text(value) if value.is_empty() => current,
            Answer::Text(value) if optional && value == "-" => String::new(),
            Answer::Text(value) => value,
        };
        if !optional && value.trim().is_empty() {
            writeln!(output, "{label} cannot be empty.")?;
            continue;
        }
        if let Err(error) = validate(&value) {
            writeln!(output, "Invalid {label}: {error}.")?;
            continue;
        }
        match target.write(&mut document.value, &value) {
            Ok(()) => return Ok(true),
            Err(error) => writeln!(output, "Invalid {label}: {error}.")?,
        }
    }
}

fn section<W: Write>(output: &mut W, title: &str) -> io::Result<()> {
    writeln!(output, "\n== {title} ==")
}

fn ask<R: BufRead, W: Write>(input: &mut R, output: &mut W, prompt: &str) -> io::Result<Answer> {
    write!(output, "{prompt}")?;
    output.flush()?;
    let mut value = String::new();
    if input.read_line(&mut value)? == 0 {
        return Ok(Answer::Cancel);
    }
    let value = value.trim().to_owned();
    if value == ":q" {
        Ok(Answer::Cancel)
    } else {
        Ok(Answer::Text(value))
    }
}

fn confirm<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    message: &str,
    default: bool,
) -> io::Result<Option<bool>> {
    let marker = if default { "Y/n" } else { "y/N" };
    loop {
        match ask(input, output, &format!("{message} [{marker}]: "))? {
            Answer::Cancel => return Ok(None),
            Answer::Text(value) if value.is_empty() => return Ok(Some(default)),
            Answer::Text(value)
                if value.eq_ignore_ascii_case("y") || value.eq_ignore_ascii_case("yes") =>
            {
                return Ok(Some(true));
            }
            Answer::Text(value)
                if value.eq_ignore_ascii_case("n") || value.eq_ignore_ascii_case("no") =>
            {
                return Ok(Some(false));
            }
            Answer::Text(_) => writeln!(output, "Enter y, n, or :q to cancel.")?,
        }
    }
}

fn parse_choices(value: &str, choices: &[String]) -> std::result::Result<Vec<String>, String> {
    let mut selected = BTreeSet::new();
    for token in value
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let index = if let Ok(number) = token.parse::<usize>() {
            number.checked_sub(1).filter(|index| *index < choices.len())
        } else {
            choices
                .iter()
                .position(|choice| choice.eq_ignore_ascii_case(token))
        }
        .ok_or_else(|| format!("'{token}' is not a listed agent"))?;
        selected.insert(index);
    }
    if selected.is_empty() {
        return Err("enter at least one number/name, 'all', or 'none'".into());
    }
    Ok(selected
        .into_iter()
        .map(|index| choices[index].clone())
        .collect())
}

fn parse_numbers(value: &str, count: usize) -> std::result::Result<Vec<usize>, String> {
    let mut selected = BTreeSet::new();
    for token in value
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let number = token
            .parse::<usize>()
            .map_err(|_| format!("'{token}' is not a number"))?;
        let index = number
            .checked_sub(1)
            .filter(|index| *index < count)
            .ok_or_else(|| format!("'{token}' is outside 1..={count}"))?;
        selected.insert(index);
    }
    if selected.is_empty() {
        return Err("enter one or more result numbers".into());
    }
    Ok(selected.into_iter().collect())
}

fn parse_action(value: &str) -> Option<(char, Option<usize>)> {
    let mut parts = value.split_whitespace();
    let action = parts.next()?;
    if action.len() != 1 {
        return None;
    }
    let action = action.chars().next()?.to_ascii_lowercase();
    let index = match parts.next() {
        Some(value) => Some(value.parse().ok()?),
        None => None,
    };
    if parts.next().is_some() {
        return None;
    }
    Some((action, index))
}

fn valid_index(index: Option<usize>, count: usize) -> Option<usize> {
    index?.checked_sub(1).filter(|index| *index < count)
}

fn registry_entries(value: &Value) -> Vec<(String, String)> {
    value["registries"]
        .as_mapping()
        .into_iter()
        .flatten()
        .filter_map(|(name, location)| Some((name.as_str()?.into(), location.as_str()?.into())))
        .collect()
}

fn skill_names(value: &Value) -> Vec<String> {
    value["skills"]
        .as_sequence()
        .into_iter()
        .flatten()
        .map(|skill| {
            let name = skill["name"].as_str().unwrap_or("(unnamed)");
            let version = skill["version"].as_str().unwrap_or("latest");
            let location = skill["path"]
                .as_str()
                .map(|path| format!("path {path}"))
                .unwrap_or_else(|| {
                    format!(
                        "{} @ {version}",
                        skill["source"].as_str().unwrap_or("default")
                    )
                });
            format!("{name} — {location}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;

    fn fresh() -> (tempfile::TempDir, Document) {
        let directory = tempfile::tempdir().unwrap();
        let document =
            Document::load(&directory.path().join("skills.yaml"), Some("demo"), false).unwrap();
        (directory, document)
    }

    fn run(document: &mut Document, answers: &str) -> (bool, String) {
        let mut input = Cursor::new(answers.as_bytes());
        let mut output = Vec::new();
        let saved = run_prompts(document, false, &mut input, &mut output).unwrap();
        (saved, String::from_utf8(output).unwrap())
    }

    #[test]
    fn sequential_defaults_review_and_save_without_terminal_controls() {
        let (_directory, mut document) = fresh();
        let (saved, output) = run(&mut document, "\n\n\n\n\n\n\n");
        assert!(saved);
        assert!(document.path.exists());
        assert!(output.contains("== Project =="));
        assert!(output.contains("== Review =="));
        assert!(output.contains("name: demo"));
        assert!(!output.contains("\u{1b}["));
    }

    #[test]
    fn cancellation_and_declined_save_leave_the_file_untouched() {
        let (_directory, mut document) = fresh();
        document.save(false).unwrap();
        let original = fs::read_to_string(&document.path).unwrap();
        document = Document::load(&document.path, None, false).unwrap();
        let (saved, _) = run(&mut document, "changed\n:q\n");
        assert!(!saved);
        assert_eq!(fs::read_to_string(&document.path).unwrap(), original);

        document = Document::load(&document.path, None, false).unwrap();
        let (saved, _) = run(&mut document, "changed\n\n\n\n\n\nn\n");
        assert!(!saved);
        assert_eq!(fs::read_to_string(&document.path).unwrap(), original);
    }

    #[test]
    fn invalid_agent_input_reprompts_and_optional_values_can_be_cleared() {
        let (_directory, mut document) = fresh();
        let (saved, output) = run(&mut document, "\n-\nmissing\n2, copilot\n\n\n\n\ny\n");
        assert!(saved);
        assert!(output.contains("Invalid selection"));
        assert!(document.value["version"].is_null());
        assert_eq!(document.value["agents"].as_sequence().unwrap().len(), 2);
    }

    #[test]
    fn registry_and_manual_skill_actions_are_named_and_preserve_metadata() {
        let (_directory, mut document) = fresh();
        document.value["extension"] = Value::String("keep".into());
        let answers = concat!(
            "\n\n\n",                               // project, version, agents
            "a\ncompany\n/registry\n\n",            // add registry, continue
            "a\nteam/spec\n\ncompany\n./local\n\n", // add skill, continue
            "\n",                                   // keep optional workspace settings
            "\n"                                    // save
        );
        let (saved, output) = run(&mut document, answers);
        assert!(saved);
        assert!(output.contains("Registry action"));
        assert!(output.contains("Skill action"));
        assert_eq!(document.value["registries"]["company"], "/registry");
        assert_eq!(document.value["skills"][0]["name"], "team/spec");
        assert_eq!(document.value["skills"][0]["path"], "./local");
        assert_eq!(document.value["extension"], "keep");
    }

    #[test]
    fn removal_confirmation_names_the_target() {
        let (_directory, mut document) = fresh();
        draft::add_entry(&mut document.value, 3);
        Target::Skill(0, "name")
            .write(&mut document.value, "team/spec")
            .unwrap();
        let answers = "\n\n\nd 1\nn\n\nd 1\nn\n\n\nn\n";
        let (saved, output) = run(&mut document, answers);
        assert!(!saved);
        assert!(output.contains("Remove registry 'default'"));
        assert!(output.contains("Remove skill"));
    }

    #[test]
    fn registry_selection_preserves_sources_and_rejects_name_collisions() {
        let (_directory, mut document) = fresh();
        document.value["skills"] =
            serde_yaml::from_str("- name: local/helper\n  path: ./helper\n  extension: keep\n")
                .unwrap();
        let original = document.value.clone();
        let collision = Entry {
            name: "local/helper".into(),
            registry: "default".into(),
            version: "v1.0.0".into(),
            description: None,
        };
        assert!(toggle_entry(&collision, &mut document.value).is_err());
        assert_eq!(document.value, original);

        let added = Entry {
            name: "team/spec".into(),
            registry: "company".into(),
            version: "v2.0.0".into(),
            description: None,
        };
        assert!(entry_matches(&added, "SPEC company v2"));
        toggle_entry(&added, &mut document.value).unwrap();
        assert_eq!(document.value["skills"][1]["source"], "company");
        assert_eq!(document.value["skills"][1]["version"], "v2.0.0");
        toggle_entry(&added, &mut document.value).unwrap();
        assert_eq!(document.value, original);
    }

    #[test]
    fn eof_is_cancellation() {
        let (_directory, mut document) = fresh();
        let (saved, _) = run(&mut document, "");
        assert!(!saved);
        assert!(!document.path.exists());
    }
}
