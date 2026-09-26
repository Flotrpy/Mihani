use git2::{Repository, StatusOptions};
use serde::Serialize;

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

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mihani-git-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
