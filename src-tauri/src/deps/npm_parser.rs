use std::collections::BTreeMap;

use serde::Deserialize;

use super::{Dependency, DependencyKind, Ecosystem};

#[derive(Debug, Default, Deserialize)]
struct PackageJson {
    #[serde(default)]
    dependencies: BTreeMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    dev_dependencies: BTreeMap<String, String>,
}

pub fn parse(contents: &str) -> Result<Vec<Dependency>, serde_json::Error> {
    let pkg: PackageJson = serde_json::from_str(contents)?;
    let mut deps = Vec::with_capacity(pkg.dependencies.len() + pkg.dev_dependencies.len());

    for (name, version_spec) in pkg.dependencies {
        deps.push(Dependency {
            name,
            version_spec,
            kind: DependencyKind::Runtime,
            ecosystem: Ecosystem::Npm,
        });
    }
    for (name, version_spec) in pkg.dev_dependencies {
        deps.push(Dependency {
            name,
            version_spec,
            kind: DependencyKind::Dev,
            ecosystem: Ecosystem::Npm,
        });
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runtime_and_dev_dependencies() {
        let json = r#"{
            "dependencies": { "react": "^18.3.1" },
            "devDependencies": { "vite": "^5.4.0" }
        }"#;

        let deps = parse(json).unwrap();

        assert_eq!(deps.len(), 2);
        assert!(deps
            .iter()
            .any(|d| d.name == "react" && d.kind == DependencyKind::Runtime));
        assert!(deps
            .iter()
            .any(|d| d.name == "vite" && d.kind == DependencyKind::Dev));
    }

    #[test]
    fn tolerates_missing_dependency_sections() {
        let deps = parse(r#"{"name": "empty"}"#).unwrap();
        assert!(deps.is_empty());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse("not json").is_err());
    }
}
