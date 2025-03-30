use std::path::PathBuf;

use git2::build::CheckoutBuilder;
use git2::{FetchOptions, RemoteCallbacks, Repository};
use tracing;

use crate::error::{AureaCoreError, Result};

/// A Git provider that manages a local clone of a Git repository.
pub struct GitProvider {
    /// The URL of the Git repository.
    repo_url: String,
    /// The branch to use.
    branch: String,
    /// The path to the working directory.
    work_dir: PathBuf,
    /// The Git repository instance.
    repo: Option<Repository>,
}

impl GitProvider {
    /// Creates a new Git provider.
    pub fn new(repo_url: String, branch: String, work_dir: PathBuf) -> Self {
        Self { repo_url, branch, work_dir, repo: None }
    }

    /// Clones the repository to the working directory.
    pub fn clone_repo(&mut self) -> Result<()> {
        if self.repo.is_some() {
            return Ok(());
        }

        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|stats| {
            tracing::debug!(
                "Transferred {} of {} objects ({} bytes)",
                stats.received_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        let repo = match Repository::clone(&self.repo_url, &self.work_dir) {
            Ok(repo) => repo,
            Err(e) => {
                return Err(AureaCoreError::Git(format!("Failed to clone repository: {}", e)))
            }
        };

        let mut checkout = CheckoutBuilder::new();
        checkout.force();

        if let Err(e) = repo.checkout_head(Some(&mut checkout)) {
            return Err(AureaCoreError::Git(format!("Failed to checkout HEAD: {}", e)));
        }

        if let Err(e) = repo.set_head(&format!("refs/heads/{}", self.branch)) {
            return Err(AureaCoreError::Git(format!(
                "Failed to set HEAD to {}: {}",
                self.branch, e
            )));
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

    /// Commits changes to the repository.
    /// This method is currently only used in tests but will be used for automated
    /// configuration updates in future implementations.
    #[cfg(test)]
    pub fn commit_changes(&self, message: &str) -> Result<()> {
        let repo = self
            .repo
            .as_ref()
            .ok_or_else(|| AureaCoreError::Git("Repository not initialized".to_string()))?;

        let signature = git2::Signature::now("AureaCore", "aureacore@example.com")
            .map_err(|e| AureaCoreError::Git(format!("Failed to create signature: {}", e)))?;

        let tree_id = repo
            .index()
            .and_then(|mut index| index.write_tree())
            .map_err(|e| AureaCoreError::Git(format!("Failed to write tree: {}", e)))?;

        let tree = repo
            .find_tree(tree_id)
            .map_err(|e| AureaCoreError::Git(format!("Failed to find tree: {}", e)))?;

        let parent = repo
            .head()
            .and_then(|head| head.peel_to_commit())
            .map_err(|e| AureaCoreError::Git(format!("Failed to get HEAD commit: {}", e)))?;

        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent])
            .map_err(|e| AureaCoreError::Git(format!("Failed to commit: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2::build::CheckoutBuilder;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    use super::*;

    fn setup_test_repo() -> (TempDir, std::path::PathBuf) {
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
        let signature = Signature::now("test", "test@example.com").unwrap();

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

    #[test]
    fn test_git_provider_sync_config() {
        let (_temp_dir, repo_path) = setup_test_repo();
        let work_dir = repo_path.parent().unwrap().join("work-dir");
        let mut provider = GitProvider::new(
            repo_path.to_str().unwrap().to_string(),
            "main".to_string(),
            work_dir.clone(),
        );

        // Initial clone
        provider.clone_repo().unwrap();

        // Simulate configuration update
        let config_dir = work_dir.join("configs");
        fs::create_dir_all(&config_dir).unwrap();
        let service_config = config_dir.join("service.yaml");
        fs::write(&service_config, "name: test-service\nversion: 1.0.0").unwrap();

        // Add and stage the new configuration
        let repo = provider.repo.as_ref().unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("configs/service.yaml")).unwrap();
        index.write().unwrap();

        // Commit the configuration update
        let result = provider.commit_changes("Add service configuration");
        assert!(result.is_ok());

        // Verify the configuration was committed
        let head = repo.head().unwrap();
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap(), "Add service configuration");

        // Verify the file exists and contains the expected content
        let config_content = fs::read_to_string(&service_config).unwrap();
        assert!(config_content.contains("name: test-service"));
    }
}
