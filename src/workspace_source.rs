use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::WalkDir;

pub(crate) fn is_git_source(source: &str) -> bool {
    source.starts_with("https://")
        || source.starts_with("ssh://")
        || source.starts_with("git@")
        || source.starts_with("file://")
}

pub(crate) fn source_integrity(
    project_root: &Path,
    relative: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let source = resolve_project_source(project_root, relative)?;
    hash_tree(&source)
}

fn resolve_project_source(
    project_root: &Path,
    relative: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = Path::new(relative);
    if relative.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("workspace source must be a repository-relative path".into());
    }
    let joined = project_root.join(path);
    let root = fs::canonicalize(project_root)?;
    let canonical = fs::canonicalize(&joined)?;
    canonical
        .strip_prefix(&root)
        .map_err(|_| "workspace source escapes the repository")?;
    if !canonical.is_dir() {
        return Err("workspace source must be a directory".into());
    }
    let mut current = project_root.to_path_buf();
    for component in path.components() {
        if let Component::Normal(part) = component {
            current.push(part);
            if fs::symlink_metadata(&current)?.file_type().is_symlink() {
                return Err("workspace source path contains a symlink".into());
            }
        }
    }
    for entry in WalkDir::new(&canonical).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() && !is_allowed_standard_pointer(&canonical, entry.path())
        {
            return Err("workspace package contains an unsafe symlink".into());
        }
    }
    Ok(canonical)
}

fn hash_tree(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() && !is_allowed_standard_pointer(path, entry.path()) {
            return Err("workspace package contains an unsafe symlink".into());
        }
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    let mut hasher = Sha256::new();
    for file in files {
        let relative = file.strip_prefix(path)?.to_string_lossy();
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        let content = fs::read(file)?;
        hasher.update((content.len() as u64).to_be_bytes());
        hasher.update(content);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn is_allowed_standard_pointer(root: &Path, path: &Path) -> bool {
    if path.parent() != Some(root) {
        return false;
    }
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !matches!(name, "default" | "latest") {
        return false;
    }
    fs::canonicalize(path)
        .ok()
        .and_then(|target| target.strip_prefix(root).ok().map(Path::to_path_buf))
        .is_some_and(|relative| {
            relative.components().count() == 1
                && relative
                    .file_name()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.starts_with('v'))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_workspace_source_has_stable_integrity() {
        let project = tempfile::tempdir().unwrap();
        let source = project.path().join("workspace/standard");
        fs::create_dir_all(source.join("v1.0.0")).unwrap();
        fs::write(source.join("AGENT_MIGRATION.md"), "migration\n").unwrap();
        fs::write(source.join("v1.0.0/manifest.yaml"), "version: 1.0.0\n").unwrap();
        let digest = source_integrity(project.path(), "workspace/standard").unwrap();
        assert_eq!(
            digest,
            "sha256:d6eb0eaf2ab0ffb48d6c1958768bd86383c0903a2bdc3611d5920f64a7717708"
        );
        assert_eq!(
            source_integrity(project.path(), "workspace/standard").unwrap(),
            digest
        );
    }

    #[test]
    fn local_workspace_source_rejects_unsafe_paths() {
        let project = tempfile::tempdir().unwrap();
        fs::create_dir(project.path().join("standard")).unwrap();
        assert!(source_integrity(project.path(), "../standard").is_err());
        assert!(source_integrity(project.path(), "/tmp/standard").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn local_workspace_source_allows_version_pointer_but_rejects_other_links() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let source = project.path().join("standard");
        fs::create_dir_all(source.join("v1.0.0")).unwrap();
        fs::write(source.join("v1.0.0/manifest.yaml"), "version: 1.0.0\n").unwrap();
        symlink("v1.0.0", source.join("default")).unwrap();
        assert!(source_integrity(project.path(), "standard").is_ok());
        symlink("v1.0.0", source.join("unexpected")).unwrap();
        assert!(source_integrity(project.path(), "standard").is_err());
        fs::remove_file(source.join("unexpected")).unwrap();
        symlink("standard", project.path().join("linked")).unwrap();
        assert!(source_integrity(project.path(), "linked").is_err());
    }
}
