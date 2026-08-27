use std::fs;
use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct DockerInfo {
    pub has_dockerfile: bool,
    pub has_compose: bool,
    pub compose_services: Vec<String>,
}

pub fn detect(project_path: &Path) -> DockerInfo {
    let has_dockerfile = project_path.join("Dockerfile").is_file();

    let compose_path = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ]
    .iter()
    .map(|name| project_path.join(name))
    .find(|path| path.is_file());

    let compose_services = compose_path
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|contents| parse_compose_services(&contents))
        .unwrap_or_default();

    DockerInfo {
        has_dockerfile,
        has_compose: compose_path.is_some(),
        compose_services,
    }
}

/// Scoped line-scan for top-level service names under a `services:` block.
/// A real compose file only nests service names one level deep, so this
/// avoids pulling in a full YAML parser for a single field.
fn parse_compose_services(contents: &str) -> Vec<String> {
    let mut services = Vec::new();
    let mut in_services_block = false;
    let mut service_indent: Option<usize> = None;

    for line in contents.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line.len() - trimmed.len();

        if !in_services_block {
            if indent == 0 && trimmed.trim_end() == "services:" {
                in_services_block = true;
            }
            continue;
        }

        if indent == 0 {
            break; // left the services block entirely
        }

        let expected = *service_indent.get_or_insert(indent);
        if indent != expected {
            continue; // a nested key under a service (ports, environment, ...)
        }

        if let Some((name, _)) = trimmed.split_once(':') {
            services.push(name.trim().to_string());
        }
    }

    services
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detects_dockerfile_only() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM node:20\n").unwrap();

        let info = detect(dir.path());

        assert!(info.has_dockerfile);
        assert!(!info.has_compose);
    }

    #[test]
    fn parses_compose_service_names() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "version: \"3\"\nservices:\n  api:\n    build: .\n    ports:\n      - \"8000:8000\"\n  redis:\n    image: redis:7\n",
        )
        .unwrap();

        let info = detect(dir.path());

        assert!(info.has_compose);
        assert_eq!(info.compose_services, vec!["api", "redis"]);
    }

    #[test]
    fn reports_no_docker_for_plain_directory() {
        let dir = tempfile::tempdir().unwrap();
        let info = detect(dir.path());
        assert!(!info.has_dockerfile);
        assert!(!info.has_compose);
        assert!(info.compose_services.is_empty());
    }
}
