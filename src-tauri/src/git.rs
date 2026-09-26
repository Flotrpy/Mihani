use git2::{Repository, Signature, StatusOptions};
use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct GitFileStatus {
    pub path: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct GitRepoStatus {
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub files: Vec<GitFileStatus>,
}

fn classify(flags: git2::Status) -> &'static str {
    if flags.is_conflicted() {
        "conflicted"
    } else if flags.is_wt_new() || flags.is_index_new() {
        "added"
    } else if flags.is_wt_deleted() || flags.is_index_deleted() {
        "deleted"
    } else if flags.is_wt_renamed() || flags.is_index_renamed() {
        "renamed"
    } else if flags.is_wt_modified() || flags.is_index_modified() {
        "modified"
    } else {
        "unknown"
    }
}

#[tauri::command]
pub fn git_status(path: String) -> Result<GitRepoStatus, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;

    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand())
        .unwrap_or("HEAD (detached)")
        .to_string();

    let mut ahead = 0;
    let mut behind = 0;
    if let Some(head_ref) = &head {
        if let Ok(local_oid) = head_ref.peel_to_commit().map(|c| c.id()) {
            let upstream_name = repo
                .branch_upstream_name(head_ref.name().unwrap_or_default())
                .ok();
            if let Some(upstream_name) = upstream_name {
                if let Some(name) = upstream_name.as_str() {
                    if let Ok(upstream_ref) = repo.find_reference(name) {
                        if let Ok(upstream_oid) = upstream_ref.peel_to_commit().map(|c| c.id()) {
                            if let Ok((a, b)) = repo.graph_ahead_behind(local_oid, upstream_oid) {
                                ahead = a;
                                behind = b;
                            }
                        }
                    }
                }
            }
        }
    }

    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let files = statuses
        .iter()
        .filter_map(|entry| {
            let path = entry.path()?.to_string();
            Some(GitFileStatus {
                path,
                status: classify(entry.status()).to_string(),
            })
        })
        .collect();

    Ok(GitRepoStatus {
        branch,
        ahead,
        behind,
        files,
    })
}

/// Returns a unified diff of the working tree against HEAD for a single
/// file, including untracked files (shown as an addition against /dev/null).
#[tauri::command]
pub fn git_diff(path: String, file: String) -> Result<String, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;

    let head_tree = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok());

    let mut opts = git2::DiffOptions::new();
    opts.pathspec(&file)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);

    let diff = repo
        .diff_tree_to_workdir(head_tree.as_ref(), Some(&mut opts))
        .map_err(|e| e.to_string())?;

    let mut buf = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let origin = line.origin();
        if origin == '+' || origin == '-' || origin == ' ' {
            buf.push(origin);
        }
        buf.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|e| e.to_string())?;

    Ok(buf)
}

/// Stages every pending change (new/modified/deleted, including untracked)
/// and creates a commit on HEAD with the given message.
#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<String, String> {
    let repo = Repository::discover(&path).map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;
    index.update_all(["*"].iter(), None).map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;

    let tree_oid = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_oid).map_err(|e| e.to_string())?;

    let signature = repo
        .signature()
        .or_else(|_| Signature::now("Mihani", "mihani@localhost"))
        .map_err(|e| e.to_string())?;

    let parents = match repo.head().ok().and_then(|h| h.peel_to_commit().ok()) {
        Some(commit) => vec![commit],
        None => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let commit_oid = repo
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &parent_refs,
        )
        .map_err(|e| e.to_string())?;

    Ok(commit_oid.to_string())
}

fn run_git(path: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Pushes the current branch. Delegates to the `git` binary so the user's
/// existing credential helper / SSH agent / gh auth is reused as-is.
#[tauri::command]
pub fn git_push(path: String) -> Result<String, String> {
    run_git(&path, &["push"])
}

#[tauri::command]
pub fn git_pull(path: String) -> Result<String, String> {
    run_git(&path, &["pull", "--ff-only"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_new_file_as_added() {
        assert_eq!(classify(git2::Status::WT_NEW), "added");
        assert_eq!(classify(git2::Status::INDEX_NEW), "added");
    }

    #[test]
    fn classifies_modified_file() {
        assert_eq!(classify(git2::Status::WT_MODIFIED), "modified");
    }

    #[test]
    fn classifies_deleted_file() {
        assert_eq!(classify(git2::Status::WT_DELETED), "deleted");
    }

    #[test]
    fn classifies_conflicted_file() {
        assert_eq!(classify(git2::Status::CONFLICTED), "conflicted");
    }

    #[test]
    fn git_status_reports_branch_and_dirty_file_on_fresh_repo() {
        let dir = tempfile_dir();
        let repo = Repository::init(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "hello").unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        let status = git_status(dir.to_string_lossy().to_string()).unwrap();
        assert_eq!(status.files.len(), 1);
        assert_eq!(status.files[0].path, "README.md");
        assert_eq!(status.files[0].status, "added");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn git_commit_stages_and_commits_all_changes() {
        let dir = tempfile_dir();
        let repo = Repository::init(&dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        std::fs::write(dir.join("README.md"), "hello").unwrap();

        let oid = git_commit(dir.to_string_lossy().to_string(), "initial commit".into())
            .unwrap();
        assert_eq!(oid.len(), 40);

        let status = git_status(dir.to_string_lossy().to_string()).unwrap();
        assert!(status.files.is_empty());

        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head_commit.message().unwrap(), "initial commit");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn git_diff_reports_added_lines_for_untracked_file() {
        let dir = tempfile_dir();
        Repository::init(&dir).unwrap();
        std::fs::write(dir.join("new.txt"), "line one\nline two\n").unwrap();

        let diff = git_diff(dir.to_string_lossy().to_string(), "new.txt".into()).unwrap();
        assert!(diff.contains("+line one"));
        assert!(diff.contains("+line two"));

        std::fs::remove_dir_all(&dir).ok();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mihani-git-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
