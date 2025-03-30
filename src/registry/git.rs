use std::path::{Path, PathBuf};

use git2::{FetchOptions, RemoteCallbacks, Repository};

use crate::error::{AureaCoreError, Result};

/// Handles Git operations for configuration management
pub struct GitProvider {
    /// URL of the Git repository
    repo_url: String,
    /// Branch to use for configurations
    branch: String,
    /// Local path where the repository is cloned
    local_path: PathBuf,
    /// The Git repository instance
    repo: Option<Repository>,
}

impl GitProvider {
    /// Create a new GitProvider
    pub fn new(repo_url: String, branch: String, local_path: impl Into<PathBuf>) -> Self {
        Self { repo_url, branch, local_path: local_path.into(), repo: None }
    }

    /// Clone or open the repository
    pub fn clone_repo(&mut self) -> Result<PathBuf> {
        if self.repo.is_some() {
            return Ok(self.local_path.clone());
        }

        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|progress| {
            tracing::debug!(
                "Git progress: {}/{} objects",
                progress.received_objects(),
                progress.total_objects()
            );
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let repo = if self.local_path.exists() {
            Repository::open(&self.local_path).map_err(|e| {
                AureaCoreError::GitOperation(format!("Failed to open repository: {}", e))
            })?
        } else {
            Repository::clone_recurse(&self.repo_url, &self.local_path).map_err(|e| {
                AureaCoreError::GitOperation(format!("Failed to clone repository: {}", e))
            })?
        };

        // Checkout the specified branch
        let obj = repo.revparse_single(&format!("origin/{}", self.branch)).map_err(|e| {
            AureaCoreError::GitOperation(format!("Failed to find branch '{}': {}", self.branch, e))
        })?;

        repo.checkout_tree(&obj, None).map_err(|e| {
            AureaCoreError::GitOperation(format!("Failed to checkout branch: {}", e))
        })?;

        self.repo = Some(repo);
        Ok(self.local_path.clone())
    }

    /// Pull latest changes from the remote repository
    pub fn pull_changes(&mut self) -> Result<()> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            AureaCoreError::GitOperation("Repository not initialized".to_string())
        })?;

        let mut remote = repo
            .find_remote("origin")
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to find remote: {}", e)))?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|progress| {
            tracing::debug!(
                "Git pull progress: {}/{} objects",
                progress.received_objects(),
                progress.total_objects()
            );
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote
            .fetch(&[&self.branch], Some(&mut fetch_options), None)
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to fetch changes: {}", e)))?;

        Ok(())
    }

    /// Commit and push changes
    pub fn commit_changes(&self, message: &str) -> Result<()> {
        let repo = self.repo.as_ref().ok_or_else(|| {
            AureaCoreError::GitOperation("Repository not initialized".to_string())
        })?;

        let mut index = repo
            .index()
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to get index: {}", e)))?;

        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).map_err(|e| {
            AureaCoreError::GitOperation(format!("Failed to add files to index: {}", e))
        })?;

        index
            .write()
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to write index: {}", e)))?;

        let tree_id = index
            .write_tree()
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to write tree: {}", e)))?;

        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to find tree: {}", e)))?;

        let signature = repo
            .signature()
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to get signature: {}", e)))?;

        let parent = repo
            .head()
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to get HEAD: {}", e)))?;

        let parent_commit = parent.peel_to_commit().map_err(|e| {
            AureaCoreError::GitOperation(format!("Failed to get parent commit: {}", e))
        })?;

        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent_commit])
            .map_err(|e| AureaCoreError::GitOperation(format!("Failed to create commit: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    fn setup_test_repo() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize a test repository
        let repo = Repository::init(&repo_path).unwrap();
        let signature = git2::Signature::now("test", "test@example.com").unwrap();

        // Create an initial commit
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_git_provider_initialization() {
        let (temp_dir, repo_path) = setup_test_repo();
        let work_dir = TempDir::new().unwrap();

        let mut provider =
            GitProvider::new(repo_path, "main".to_string(), work_dir.path().to_path_buf());

        assert!(provider.repo.is_none());
        assert_eq!(provider.branch, "main");
    }

    #[test]
    fn test_git_provider_clone_existing_repo() {
        let (temp_dir, repo_path) = setup_test_repo();
        let work_dir = TempDir::new().unwrap();

        let mut provider =
            GitProvider::new(repo_path, "main".to_string(), work_dir.path().to_path_buf());

        let result = provider.clone_repo();
        assert!(result.is_ok());
        assert!(provider.repo.is_some());
    }
}
