use std::path::Path;
use std::process::Command;

use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Serialize)]
pub struct FileChange {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
    pub untracked: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Shells out to the system `git` binary rather than vendoring libgit2.
/// Forgeyard already assumes a working git install on the host (it is a
/// developer tool), porcelain output is a stable, documented interface,
/// and this avoids a slow C build for functionality most projects use a
/// handful of times a minute at most.
fn run_git(repo_path: &Path, args: &[&str]) -> AppResult<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| AppError::Git(format!("failed to run git: {e}")))?;

    if !output.status.success() {
        return Err(AppError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn status(repo_path: &Path) -> AppResult<GitStatus> {
    let output = run_git(repo_path, &["status", "--porcelain=v2", "--branch"])?;
    let mut status = GitStatus::default();

    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            if rest != "(detached)" {
                status.branch = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    status.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    status.behind = n.parse().unwrap_or(0);
                }
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            parse_ordinary_entry(rest, &mut status.staged, &mut status.unstaged);
        } else if let Some(rest) = line.strip_prefix("2 ") {
            parse_rename_entry(rest, &mut status.staged, &mut status.unstaged);
        } else if let Some(rest) = line.strip_prefix("u ") {
            if let Some(path) = rest.split_whitespace().last() {
                status.unstaged.push(FileChange {
                    path: path.to_string(),
                    status: "U".to_string(),
                });
            }
        } else if let Some(path) = line.strip_prefix("? ") {
            status.untracked.push(path.to_string());
        }
    }

    Ok(status)
}

/// Porcelain v2 "ordinary" changed-entry: `XY sub mH mI mW hH hI path`.
fn parse_ordinary_entry(rest: &str, staged: &mut Vec<FileChange>, unstaged: &mut Vec<FileChange>) {
    let fields: Vec<&str> = rest.splitn(8, ' ').collect();
    let (Some(xy), Some(path)) = (fields.first(), fields.get(7)) else {
        return;
    };
    push_xy(xy, path, staged, unstaged);
}

/// Porcelain v2 "rename/copy" entry: `XY sub mH mI mW hH hI score path<TAB>origPath`.
fn parse_rename_entry(rest: &str, staged: &mut Vec<FileChange>, unstaged: &mut Vec<FileChange>) {
    let fields: Vec<&str> = rest.splitn(9, ' ').collect();
    let (Some(xy), Some(path_field)) = (fields.first(), fields.get(8)) else {
        return;
    };
    let path = path_field.split('\t').next().unwrap_or(path_field);
    push_xy(xy, path, staged, unstaged);
}

fn push_xy(xy: &str, path: &str, staged: &mut Vec<FileChange>, unstaged: &mut Vec<FileChange>) {
    let mut chars = xy.chars();
    let (Some(x), Some(y)) = (chars.next(), chars.next()) else {
        return;
    };
    if x != '.' {
        staged.push(FileChange { path: path.to_string(), status: x.to_string() });
    }
    if y != '.' {
        unstaged.push(FileChange { path: path.to_string(), status: y.to_string() });
    }
}

pub fn branches(repo_path: &Path) -> AppResult<Vec<BranchInfo>> {
    let output = run_git(
        repo_path,
        &["branch", "--all", "--format=%(refname:short)%00%(HEAD)"],
    )?;

    let branches = output
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\0');
            let name = parts.next()?.to_string();
            let is_current = parts.next() == Some("*");
            let is_remote = name.starts_with("origin/") || name.contains("/HEAD");
            Some(BranchInfo { name, is_current, is_remote })
        })
        .collect();

    Ok(branches)
}

pub fn checkout(repo_path: &Path, branch: &str, create: bool) -> AppResult<()> {
    if create {
        run_git(repo_path, &["checkout", "-b", branch])?;
    } else {
        run_git(repo_path, &["checkout", branch])?;
    }
    Ok(())
}

pub fn stage(repo_path: &Path, files: &[String]) -> AppResult<()> {
    let mut args = vec!["add", "--"];
    args.extend(files.iter().map(String::as_str));
    run_git(repo_path, &args)?;
    Ok(())
}

pub fn unstage(repo_path: &Path, files: &[String]) -> AppResult<()> {
    let mut args = vec!["restore", "--staged", "--"];
    args.extend(files.iter().map(String::as_str));
    run_git(repo_path, &args)?;
    Ok(())
}

pub fn commit(repo_path: &Path, message: &str) -> AppResult<()> {
    run_git(repo_path, &["commit", "-m", message])?;
    Ok(())
}

pub fn diff(repo_path: &Path, file: Option<&str>, staged: bool) -> AppResult<String> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--staged");
    }
    if let Some(f) = file {
        args.push("--");
        args.push(f);
    }
    run_git(repo_path, &args)
}

pub fn log(repo_path: &Path, limit: u32) -> AppResult<Vec<CommitInfo>> {
    let limit_arg = format!("-{limit}");
    // Unit/record separators avoid ambiguity with punctuation in commit
    // messages, which a naive delimiter like "|" or "," would not.
    let output = run_git(
        repo_path,
        &[
            "log",
            &limit_arg,
            "--pretty=format:%H%x1f%h%x1f%an%x1f%aI%x1f%s%x1e",
        ],
    )?;

    let commits = output
        .split('\u{1e}')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let mut fields = entry.split('\u{1f}');
            Some(CommitInfo {
                hash: fields.next()?.to_string(),
                short_hash: fields.next()?.to_string(),
                author: fields.next()?.to_string(),
                date: fields.next()?.to_string(),
                message: fields.next()?.to_string(),
            })
        })
        .collect();

    Ok(commits)
}

pub fn stash(repo_path: &Path, message: Option<&str>) -> AppResult<()> {
    let mut args = vec!["stash", "push"];
    if let Some(m) = message {
        args.push("-m");
        args.push(m);
    }
    run_git(repo_path, &args)?;
    Ok(())
}

pub fn stash_pop(repo_path: &Path) -> AppResult<()> {
    run_git(repo_path, &["stash", "pop"])?;
    Ok(())
}

pub fn merge(repo_path: &Path, branch: &str) -> AppResult<()> {
    run_git(repo_path, &["merge", "--no-edit", branch])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Real git repo in a temp dir, with local (not global) identity so the
    /// test doesn't depend on the host's git config.
    fn init_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        run_git(dir.path(), &["init", "--initial-branch=main"]).unwrap();
        run_git(dir.path(), &["config", "user.email", "test@example.com"]).unwrap();
        run_git(dir.path(), &["config", "user.name", "Test User"]).unwrap();
        dir
    }

    fn commit_file(dir: &Path, name: &str, contents: &str, message: &str) {
        fs::write(dir.join(name), contents).unwrap();
        run_git(dir, &["add", "--", name]).unwrap();
        run_git(dir, &["commit", "-m", message]).unwrap();
    }

    #[test]
    fn status_reports_branch_and_untracked_files() {
        let repo = init_repo();
        commit_file(repo.path(), "README.md", "hello\n", "initial commit");
        fs::write(repo.path().join("new.txt"), "new\n").unwrap();

        let status = status(repo.path()).unwrap();

        assert_eq!(status.branch.as_deref(), Some("main"));
        assert_eq!(status.untracked, vec!["new.txt"]);
        assert!(status.staged.is_empty());
    }

    #[test]
    fn status_splits_staged_and_unstaged_changes() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");

        fs::write(repo.path().join("a.txt"), "two\n").unwrap();
        stage(repo.path(), &["a.txt".to_string()]).unwrap();
        fs::write(repo.path().join("a.txt"), "three\n").unwrap();

        let status = status(repo.path()).unwrap();

        assert_eq!(status.staged.len(), 1);
        assert_eq!(status.staged[0].status, "M");
        assert_eq!(status.unstaged.len(), 1);
        assert_eq!(status.unstaged[0].status, "M");
    }

    #[test]
    fn commit_creates_a_new_log_entry() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");

        fs::write(repo.path().join("b.txt"), "two\n").unwrap();
        stage(repo.path(), &["b.txt".to_string()]).unwrap();
        commit(repo.path(), "add b").unwrap();

        let commits = log(repo.path(), 10).unwrap();

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "add b");
        assert_eq!(commits[1].message, "initial commit");
    }

    #[test]
    fn checkout_create_switches_to_a_new_branch() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");

        checkout(repo.path(), "feature/x", true).unwrap();
        let status = status(repo.path()).unwrap();

        assert_eq!(status.branch.as_deref(), Some("feature/x"));
    }

    #[test]
    fn stash_push_and_pop_round_trip_a_change() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");
        fs::write(repo.path().join("a.txt"), "changed\n").unwrap();

        stash(repo.path(), Some("wip")).unwrap();
        assert!(status(repo.path()).unwrap().unstaged.is_empty());

        stash_pop(repo.path()).unwrap();
        assert_eq!(status(repo.path()).unwrap().unstaged.len(), 1);
    }

    #[test]
    fn merge_brings_in_changes_from_another_branch() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");
        checkout(repo.path(), "feature", true).unwrap();
        commit_file(repo.path(), "b.txt", "two\n", "add b");
        checkout(repo.path(), "main", false).unwrap();

        merge(repo.path(), "feature").unwrap();

        assert!(repo.path().join("b.txt").exists());
    }

    #[test]
    fn diff_shows_unstaged_changes() {
        let repo = init_repo();
        commit_file(repo.path(), "a.txt", "one\n", "initial commit");
        fs::write(repo.path().join("a.txt"), "two\n").unwrap();

        let diff = diff(repo.path(), None, false).unwrap();

        assert!(diff.contains("-one"));
        assert!(diff.contains("+two"));
    }
}
