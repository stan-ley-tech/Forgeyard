use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
    Pip,
    Poetry,
    Pipenv,
    Cargo,
    Go,
}

/// Prefers lockfiles over manifests since the lockfile is the ground truth
/// for which package manager actually installed the dependency tree.
pub fn detect(project_path: &Path) -> Vec<PackageManager> {
    let mut found = Vec::new();
    let exists = |name: &str| project_path.join(name).is_file();

    if exists("pnpm-lock.yaml") {
        found.push(PackageManager::Pnpm);
    } else if exists("yarn.lock") {
        found.push(PackageManager::Yarn);
    } else if exists("bun.lockb") {
        found.push(PackageManager::Bun);
    } else if exists("package-lock.json") || exists("package.json") {
        found.push(PackageManager::Npm);
    }

    if exists("poetry.lock") {
        found.push(PackageManager::Poetry);
    } else if exists("Pipfile") {
        found.push(PackageManager::Pipenv);
    } else if exists("requirements.txt") || exists("pyproject.toml") {
        found.push(PackageManager::Pip);
    }

    if exists("Cargo.toml") {
        found.push(PackageManager::Cargo);
    }
    if exists("go.mod") {
        found.push(PackageManager::Go);
    }

    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn prefers_lockfile_over_bare_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();

        assert_eq!(detect(dir.path()), vec![PackageManager::Pnpm]);
    }

    #[test]
    fn falls_back_to_npm_without_a_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        assert_eq!(detect(dir.path()), vec![PackageManager::Npm]);
    }

    #[test]
    fn detects_poetry_over_plain_pip() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("pyproject.toml"), "[tool.poetry]\n").unwrap();
        fs::write(dir.path().join("poetry.lock"), "").unwrap();

        assert_eq!(detect(dir.path()), vec![PackageManager::Poetry]);
    }
}
