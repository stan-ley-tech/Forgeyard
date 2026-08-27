use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Node,
    Python,
    Go,
    Rust,
}

/// Marker-file based detection. Kept to the ecosystems Forgeyard actually
/// parses dependencies for, rather than guessing at every language a repo
/// might touch.
pub fn detect(project_path: &Path) -> Vec<Language> {
    let mut found = Vec::new();

    if project_path.join("package.json").is_file() {
        found.push(Language::Node);
    }
    if project_path.join("requirements.txt").is_file() || project_path.join("pyproject.toml").is_file() {
        found.push(Language::Python);
    }
    if project_path.join("go.mod").is_file() {
        found.push(Language::Go);
    }
    if project_path.join("Cargo.toml").is_file() {
        found.push(Language::Rust);
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_multiple_languages_in_a_polyglot_repo() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("go.mod"), "module example\n").unwrap();

        let found = detect(dir.path());

        assert_eq!(found, vec![Language::Node, Language::Go]);
    }

    #[test]
    fn detects_python_from_either_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[project]\nname = \"x\"\n").unwrap();

        assert_eq!(detect(dir.path()), vec![Language::Python]);
    }

    #[test]
    fn returns_empty_for_unrecognized_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(detect(dir.path()).is_empty());
    }
}
