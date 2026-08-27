use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;

use super::{toml_value_to_spec, Dependency, DependencyKind, Ecosystem};

fn requirement_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([A-Za-z0-9_.\-]+)\s*(?:\[[^\]]*\])?\s*(.*)$").unwrap())
}

fn split_requirement(raw: &str) -> Option<(String, String)> {
    let caps = requirement_re().captures(raw.trim())?;
    let name = caps.get(1)?.as_str().to_string();
    let version_spec = caps
        .get(2)
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "*".to_string());
    Some((name, version_spec))
}

/// Parses a `requirements.txt`. Option lines (`-r other.txt`, `--index-url
/// ...`, editable/VCS installs starting with `-e`) are skipped rather than
/// misread as package names.
pub fn parse_requirements(contents: &str) -> Vec<Dependency> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('-'))
        .filter_map(split_requirement)
        .map(|(name, version_spec)| Dependency {
            name,
            version_spec,
            kind: DependencyKind::Runtime,
            ecosystem: Ecosystem::Pip,
        })
        .collect()
}

#[derive(Debug, Default, Deserialize)]
struct PyProject {
    #[serde(default)]
    project: Option<ProjectTable>,
    #[serde(default)]
    tool: Option<ToolTable>,
}

#[derive(Debug, Default, Deserialize)]
struct ProjectTable {
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default, rename = "optional-dependencies")]
    optional_dependencies: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct ToolTable {
    #[serde(default)]
    poetry: Option<PoetryTable>,
}

#[derive(Debug, Default, Deserialize)]
struct PoetryTable {
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, toml::Value>,
    #[serde(default)]
    group: BTreeMap<String, PoetryGroup>,
}

#[derive(Debug, Default, Deserialize)]
struct PoetryGroup {
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
}

/// Understands both PEP 621 (`[project.dependencies]`) and Poetry-style
/// (`[tool.poetry.dependencies]`, including dependency groups) manifests,
/// since real-world `pyproject.toml` files use either.
pub fn parse_pyproject(contents: &str) -> Result<Vec<Dependency>, toml::de::Error> {
    let doc: PyProject = toml::from_str(contents)?;
    let mut deps = Vec::new();

    if let Some(project) = &doc.project {
        for raw in &project.dependencies {
            if let Some((name, version_spec)) = split_requirement(raw) {
                deps.push(Dependency {
                    name,
                    version_spec,
                    kind: DependencyKind::Runtime,
                    ecosystem: Ecosystem::Pip,
                });
            }
        }
        for group in project.optional_dependencies.values() {
            for raw in group {
                if let Some((name, version_spec)) = split_requirement(raw) {
                    deps.push(Dependency {
                        name,
                        version_spec,
                        kind: DependencyKind::Dev,
                        ecosystem: Ecosystem::Pip,
                    });
                }
            }
        }
    }

    if let Some(poetry) = doc.tool.as_ref().and_then(|t| t.poetry.as_ref()) {
        for (name, value) in &poetry.dependencies {
            if name == "python" {
                continue;
            }
            deps.push(Dependency {
                name: name.clone(),
                version_spec: toml_value_to_spec(value),
                kind: DependencyKind::Runtime,
                ecosystem: Ecosystem::Pip,
            });
        }
        for (name, value) in &poetry.dev_dependencies {
            deps.push(Dependency {
                name: name.clone(),
                version_spec: toml_value_to_spec(value),
                kind: DependencyKind::Dev,
                ecosystem: Ecosystem::Pip,
            });
        }
        for group in poetry.group.values() {
            for (name, value) in &group.dependencies {
                deps.push(Dependency {
                    name: name.clone(),
                    version_spec: toml_value_to_spec(value),
                    kind: DependencyKind::Dev,
                    ecosystem: Ecosystem::Pip,
                });
            }
        }
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pinned_and_ranged_requirements() {
        let deps = parse_requirements(
            "fastapi==0.116.0\nsqlalchemy>=2.0,<3.0\n# comment\n\n-r other.txt\nrequests\n",
        );

        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0].name, "fastapi");
        assert_eq!(deps[0].version_spec, "==0.116.0");
        assert_eq!(deps[2].version_spec, "*");
    }

    #[test]
    fn parses_pep621_dependencies() {
        let toml = r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = ["fastapi>=0.100", "redis[hiredis]~=5.0"]

[project.optional-dependencies]
dev = ["pytest>=8.0"]
"#;
        let deps = parse_pyproject(toml).unwrap();

        assert_eq!(deps.len(), 3);
        assert!(deps.iter().any(|d| d.name == "redis" && d.version_spec == "~=5.0"));
        assert!(deps
            .iter()
            .any(|d| d.name == "pytest" && d.kind == DependencyKind::Dev));
    }

    #[test]
    fn parses_poetry_dependencies_and_groups() {
        let toml = r#"
[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.116"
sqlalchemy = { version = "^2.0", extras = ["asyncio"] }

[tool.poetry.group.dev.dependencies]
pytest = "^8.0"
"#;
        let deps = parse_pyproject(toml).unwrap();

        assert_eq!(deps.len(), 3);
        assert!(!deps.iter().any(|d| d.name == "python"));
        assert!(deps.iter().any(|d| d.name == "sqlalchemy" && d.version_spec == "^2.0"));
        assert!(deps.iter().any(|d| d.name == "pytest" && d.kind == DependencyKind::Dev));
    }
}
