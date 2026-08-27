mod cargo_parser;
mod go_parser;
mod npm_parser;
mod python_parser;

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Npm,
    Pip,
    Go,
    Cargo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyKind {
    Runtime,
    Dev,
    Build,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dependency {
    pub name: String,
    pub version_spec: String,
    pub kind: DependencyKind,
    pub ecosystem: Ecosystem,
}

#[derive(Debug, Clone, Serialize)]
pub struct DependencyConflict {
    pub name: String,
    /// (manifest file, version spec) pairs that disagree on a version.
    pub sources: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct DependencyReport {
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<DependencyConflict>,
}

/// Reads every manifest Forgeyard understands at the project root and
/// merges them into one report. Manifests that don't exist or fail to
/// parse are skipped rather than failing the whole scan — a project with a
/// slightly malformed `requirements.txt` should still show its npm deps.
pub fn scan(project_path: &Path) -> DependencyReport {
    let mut sources: Vec<(String, Vec<Dependency>)> = Vec::new();

    if let Some(deps) = read_and_parse(project_path, "package.json", npm_parser::parse) {
        sources.push(("package.json".to_string(), deps));
    }
    if let Ok(contents) = std::fs::read_to_string(project_path.join("requirements.txt")) {
        sources.push((
            "requirements.txt".to_string(),
            python_parser::parse_requirements(&contents),
        ));
    }
    if let Some(deps) = read_and_parse(
        project_path,
        "pyproject.toml",
        python_parser::parse_pyproject,
    ) {
        sources.push(("pyproject.toml".to_string(), deps));
    }
    if let Ok(contents) = std::fs::read_to_string(project_path.join("go.mod")) {
        sources.push(("go.mod".to_string(), go_parser::parse(&contents)));
    }
    if let Some(deps) = read_and_parse(project_path, "Cargo.toml", cargo_parser::parse) {
        sources.push(("Cargo.toml".to_string(), deps));
    }

    let conflicts = find_conflicts(&sources);
    let dependencies = sources.into_iter().flat_map(|(_, deps)| deps).collect();

    DependencyReport {
        dependencies,
        conflicts,
    }
}

fn read_and_parse<T, E>(
    project_path: &Path,
    file_name: &str,
    parse: impl FnOnce(&str) -> Result<T, E>,
) -> Option<T> {
    let contents = std::fs::read_to_string(project_path.join(file_name)).ok()?;
    parse(&contents).ok()
}

/// Flags a package name as conflicting when it resolves to more than one
/// distinct version spec across the manifests scanned for this project
/// (e.g. `requirements.txt` and `pyproject.toml` disagreeing, or a package
/// pinned differently as a runtime vs. dev dependency in the same file).
fn find_conflicts(sources: &[(String, Vec<Dependency>)]) -> Vec<DependencyConflict> {
    let mut by_name: HashMap<String, Vec<(String, String)>> = HashMap::new();

    for (source, deps) in sources {
        for dep in deps {
            by_name
                .entry(dep.name.clone())
                .or_default()
                .push((source.clone(), dep.version_spec.clone()));
        }
    }

    let mut conflicts: Vec<DependencyConflict> = by_name
        .into_iter()
        .filter_map(|(name, entries)| {
            let mut distinct: Vec<&str> = entries.iter().map(|(_, v)| v.as_str()).collect();
            distinct.sort_unstable();
            distinct.dedup();
            (distinct.len() > 1).then_some(DependencyConflict {
                name,
                sources: entries,
            })
        })
        .collect();

    conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    conflicts
}

/// Shared by the Cargo.toml and pyproject.toml (Poetry) parsers: both
/// express a dependency either as a bare version string or as a table with
/// a `version` key.
fn toml_value_to_spec(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("*")
            .to_string(),
        _ => "*".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_merges_multiple_manifests() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies": {"react": "^18.0.0"}}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("go.mod"),
            "module x\n\nrequire github.com/a/b v1.0.0\n",
        )
        .unwrap();

        let report = scan(dir.path());

        assert_eq!(report.dependencies.len(), 2);
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn scan_flags_conflicting_versions_across_manifests() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "requests==2.31.0\n").unwrap();
        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"x\"\nversion = \"0\"\ndependencies = [\"requests==2.28.0\"]\n",
        )
        .unwrap();

        let report = scan(dir.path());

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].name, "requests");
    }
}
