use std::collections::BTreeMap;

use serde::Deserialize;

use super::{toml_value_to_spec, Dependency, DependencyKind, Ecosystem};

#[derive(Debug, Default, Deserialize)]
struct CargoToml {
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
    #[serde(default, rename = "dev-dependencies")]
    dev_dependencies: BTreeMap<String, toml::Value>,
    #[serde(default, rename = "build-dependencies")]
    build_dependencies: BTreeMap<String, toml::Value>,
}

pub fn parse(contents: &str) -> Result<Vec<Dependency>, toml::de::Error> {
    let doc: CargoToml = toml::from_str(contents)?;
    let mut deps = Vec::new();

    for (name, value) in &doc.dependencies {
        deps.push(Dependency {
            name: name.clone(),
            version_spec: toml_value_to_spec(value),
            kind: DependencyKind::Runtime,
            ecosystem: Ecosystem::Cargo,
        });
    }
    for (name, value) in &doc.dev_dependencies {
        deps.push(Dependency {
            name: name.clone(),
            version_spec: toml_value_to_spec(value),
            kind: DependencyKind::Dev,
            ecosystem: Ecosystem::Cargo,
        });
    }
    for (name, value) in &doc.build_dependencies {
        deps.push(Dependency {
            name: name.clone(),
            version_spec: toml_value_to_spec(value),
            kind: DependencyKind::Build,
            ecosystem: Ecosystem::Cargo,
        });
    }

    Ok(deps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_and_table_dependencies() {
        let toml = r#"
[package]
name = "demo"
version = "0.1.0"

[dependencies]
serde = { version = "1", features = ["derive"] }
anyhow = "1"

[dev-dependencies]
tempfile = "3"
"#;
        let deps = parse(toml).unwrap();

        assert_eq!(deps.len(), 3);
        assert!(deps
            .iter()
            .any(|d| d.name == "serde" && d.version_spec == "1" && d.kind == DependencyKind::Runtime));
        assert!(deps
            .iter()
            .any(|d| d.name == "tempfile" && d.kind == DependencyKind::Dev));
    }

    #[test]
    fn defaults_workspace_inherited_versions_to_wildcard() {
        let toml = "[dependencies]\nserde = { workspace = true }\n";
        let deps = parse(toml).unwrap();
        assert_eq!(deps[0].version_spec, "*");
    }
}
