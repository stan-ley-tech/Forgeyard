mod engine;

#[allow(unused_imports)] // FileChange rides along inside GitStatus for the frontend
pub use engine::{
    branches, checkout, commit, diff, log, merge, stage, stash, stash_pop, status, unstage,
    BranchInfo, CommitInfo, FileChange, GitStatus,
};
