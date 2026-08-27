mod docker;
mod env_files;
mod language;
mod package_manager;

use std::path::Path;

use serde::Serialize;

pub use docker::DockerInfo;
pub use language::Language;
pub use package_manager::PackageManager;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectHealth {
    pub languages: Vec<Language>,
    pub package_managers: Vec<PackageManager>,
    pub docker: DockerInfo,
    pub env_files: Vec<EnvFileInfo>,
    pub has_git: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvFileInfo {
    pub file_name: String,
    /// Names of the keys defined in the file. Values are never read into
    /// memory for display — this is a health signal, not a secrets viewer.
    pub key_count: usize,
}

/// Runs every detector over a project's top-level directory and combines
/// the results into a single health snapshot.
pub fn scan(project_path: &Path) -> ProjectHealth {
    ProjectHealth {
        languages: language::detect(project_path),
        package_managers: package_manager::detect(project_path),
        docker: docker::detect(project_path),
        env_files: env_files::detect(project_path),
        has_git: project_path.join(".git").exists(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_reports_has_git_when_git_dir_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();

        let health = scan(dir.path());

        assert!(health.has_git);
    }

    #[test]
    fn scan_reports_no_git_for_plain_directory() {
        let dir = tempfile::tempdir().unwrap();
        let health = scan(dir.path());
        assert!(!health.has_git);
    }
}
