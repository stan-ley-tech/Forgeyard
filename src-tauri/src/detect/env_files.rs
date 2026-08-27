use std::fs;
use std::path::Path;

use super::EnvFileInfo;

const CANDIDATES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    ".env.test",
    ".env.example",
    ".env.sample",
];

/// Reports which env files exist and how many keys they define. Values are
/// never read into the returned struct — this is a presence/shape check,
/// not a way to surface secrets in the UI.
pub fn detect(project_path: &Path) -> Vec<EnvFileInfo> {
    CANDIDATES
        .iter()
        .filter_map(|name| {
            let path = project_path.join(name);
            if !path.is_file() {
                return None;
            }
            let key_count = fs::read_to_string(&path)
                .map(|contents| count_keys(&contents))
                .unwrap_or(0);
            Some(EnvFileInfo {
                file_name: name.to_string(),
                key_count,
            })
        })
        .collect()
}

fn count_keys(contents: &str) -> usize {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && line.contains('='))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn counts_keys_while_ignoring_comments_and_blanks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(".env"),
            "# comment\nDATABASE_URL=postgres://localhost\n\nPORT=3000\n",
        )
        .unwrap();

        let files = detect(dir.path());

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name, ".env");
        assert_eq!(files[0].key_count, 2);
    }

    #[test]
    fn finds_multiple_env_variants() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".env"), "A=1\n").unwrap();
        fs::write(dir.path().join(".env.example"), "A=\n").unwrap();

        assert_eq!(detect(dir.path()).len(), 2);
    }
}
