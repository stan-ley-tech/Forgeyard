use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TaskEntry {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TaskGroup {
    pub name: String,
    #[serde(default)]
    pub tasks: Vec<TaskEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct TaskFile {
    #[serde(default)]
    groups: Vec<TaskGroup>,
}

/// Loads a project's runnable tasks: explicit groups from `forgeyard.toml`
/// plus, when present, an auto-derived "Scripts" group from `package.json`
/// so a project works out of the box without any Forgeyard-specific config.
pub fn load(project_path: &Path) -> Vec<TaskGroup> {
    let mut groups = read_forgeyard_toml(project_path).unwrap_or_default();

    if let Some(scripts_group) = derive_npm_scripts(project_path) {
        let already_defined = groups
            .iter()
            .any(|g| g.name.eq_ignore_ascii_case(&scripts_group.name));
        if !already_defined {
            groups.push(scripts_group);
        }
    }

    groups
}

fn read_forgeyard_toml(project_path: &Path) -> Option<Vec<TaskGroup>> {
    let contents = fs::read_to_string(project_path.join("forgeyard.toml")).ok()?;
    let file: TaskFile = toml::from_str(&contents).ok()?;
    Some(file.groups)
}

fn derive_npm_scripts(project_path: &Path) -> Option<TaskGroup> {
    let contents = fs::read_to_string(project_path.join("package.json")).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&contents).ok()?;
    let scripts = pkg.get("scripts")?.as_object()?;

    let tasks: Vec<TaskEntry> = scripts
        .iter()
        .filter_map(|(name, command)| {
            command.as_str()?;
            Some(TaskEntry {
                name: name.clone(),
                command: format!("npm run {name}"),
            })
        })
        .collect();

    if tasks.is_empty() {
        None
    } else {
        Some(TaskGroup {
            name: "Scripts".to_string(),
            tasks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_explicit_groups_from_forgeyard_toml() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("forgeyard.toml"),
            r#"
[[groups]]
name = "Development"
tasks = [
  { name = "Start", command = "npm run dev" },
  { name = "Test", command = "npm test" },
]

[[groups]]
name = "Database"
tasks = [
  { name = "Migrate", command = "npm run db:migrate" },
]
"#,
        )
        .unwrap();

        let groups = load(dir.path());

        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Development");
        assert_eq!(groups[0].tasks.len(), 2);
        assert_eq!(groups[1].tasks[0].command, "npm run db:migrate");
    }

    #[test]
    fn derives_a_scripts_group_from_package_json_when_no_config_exists() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"build": "vite build", "test": "vitest run"}}"#,
        )
        .unwrap();

        let groups = load(dir.path());

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "Scripts");
        assert_eq!(groups[0].tasks.len(), 2);
        assert!(groups[0].tasks.iter().any(|t| t.command == "npm run build"));
    }

    #[test]
    fn explicit_scripts_group_is_not_overridden_by_package_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("forgeyard.toml"),
            r#"
[[groups]]
name = "Scripts"
tasks = [{ name = "Custom", command = "echo hi" }]
"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"build": "vite build"}}"#,
        )
        .unwrap();

        let groups = load(dir.path());

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].tasks[0].name, "Custom");
    }

    #[test]
    fn returns_empty_when_nothing_is_configured() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).is_empty());
    }
}
