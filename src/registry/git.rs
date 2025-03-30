use std::path::PathBuf;

use git2::build::CheckoutBuilder;
use git2::{FetchOptions, RemoteCallbacks, Repository};

use crate::error::{AureaCoreError, Result};

/// Handles Git operations for configuration management
pub struct GitProvider {
    repo: Option<Repository>,
    repo_url: String,
    branch: String,
    work_dir: PathBuf,
}

impl GitProvider {
    /// Creates a new Git provider instance
    pub fn new(repo_url: String, branch: String, work_dir: PathBuf) -> Self {
        Self { repo: None, repo_url, branch, work_dir }
    }

    /// Clones the repository to the working directory
    pub fn clone_repo(&mut self) -> Result<()> {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|stats| {
            tracing::debug!(
                "Received {} of {} objects ({} bytes)",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let repo = Repository::clone(&self.repo_url, &self.work_dir)?;
        {
            let (object, reference) = repo.revparse_ext(&self.branch)?;

            let mut checkout = CheckoutBuilder::new();
            checkout.force();

            if let Some(reference) = reference {
                repo.set_head(reference.name().unwrap_or("HEAD"))?;
            } else {
                repo.set_head_detached(object.id())?;
            }

            repo.checkout_head(Some(&mut checkout))?;
        }
        self.repo = Some(repo);
        Ok(())
    }

    /// Updates the repository by pulling the latest changes
    pub fn pull(&mut self) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AureaCoreError::Git("Repository not initialized".to_string()))?;

        let mut remote = repo.find_remote("origin")?;
        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|stats| {
            tracing::debug!(
                "Received {} of {} objects ({} bytes)",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(&[&self.branch], Some(&mut fetch_options), None)?;

        let fetch_head = repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
        let commit = repo.find_commit(fetch_commit.id())?;

        let mut checkout = CheckoutBuilder::new();
        checkout.force();

        repo.checkout_tree(commit.as_object(), Some(&mut checkout))?;
        repo.set_head(format!("refs/heads/{}", self.branch).as_str())?;

        Ok(())
    }

    /// Commits changes to the repository
    #[cfg_attr(test, allow(dead_code))]
    pub fn commit_changes(&self, message: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AureaCoreError::Git("Repository not initialized".to_string()))?;

        let mut index = repo.index()?;
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let signature = git2::Signature::now("AureaCore", "aureacore@example.com")?;
        let parent = repo.head()?.peel_to_commit()?;

        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().join("test-repo");
        let repo = Repository::init(&repo_path).unwrap();

        // Create an initial commit
        fs::create_dir_all(&repo_path).unwrap();
        let readme_path = repo_path.join("README.md");
        fs::write(&readme_path, "# Test Repository").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let signature = git2::Signature::now("test", "test@example.com").unwrap();

        // Create initial commit
        repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

        // Create main branch
        let mut checkout = CheckoutBuilder::new();
        checkout.force();
        repo.checkout_head(Some(&mut checkout)).unwrap();

        // Set HEAD to refs/heads/main
        repo.set_head("refs/heads/main").unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_git_provider_initialization() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let provider = GitProvider::new(
            repo_path.to_str().unwrap().to_string(),
            "main".to_string(),
            repo_path.parent().unwrap().join("work-dir"),
        );
        assert!(provider.repo.is_none());
    }

    #[test]
    fn test_git_provider_clone_existing_repo() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let work_dir = repo_path.parent().unwrap().join("work-dir");
        let mut provider = GitProvider::new(
            repo_path.to_str().unwrap().to_string(),
            "main".to_string(),
            work_dir.clone(),
        );
        let result = provider.clone_repo();
        assert!(result.is_ok());
        assert!(work_dir.join(".git").exists());
    }

    #[test]
    fn test_git_provider_commit_changes() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let work_dir = repo_path.parent().unwrap().join("work-dir");
        let mut provider = GitProvider::new(
            repo_path.to_str().unwrap().to_string(),
            "main".to_string(),
            work_dir.clone(),
        );

        // Clone the repository
        provider.clone_repo().unwrap();

        // Create a new file
        let test_file = work_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        // Add and commit the file
        let repo = provider.repo.as_ref().unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let result = provider.commit_changes("Add test file");
        assert!(result.is_ok());

        // Verify the commit
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Add test file");
    }
}
