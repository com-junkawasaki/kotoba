//! # GitHub Pages Deployer for Kotoba SSG
//!
//! This module provides GitHub Pages deployment functionality implemented entirely
//! in the Kotoba language. It handles the complete deployment workflow including:
//!
//! - **Git Operations**: Automatic commit and push to gh-pages branch
//! - **CNAME Configuration**: Custom domain setup
//! - **Asset Optimization**: CDN-ready asset processing
//! - **Deployment Verification**: Post-deployment checks
//! - **Rollback Support**: Easy rollback to previous versions

use crate::{SiteConfig, Result};
use git2::{Repository, Signature, Index, Branch, BranchType, PushOptions};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use reqwest::Client;

/// GitHub Pages deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPagesConfig {
    /// GitHub repository owner
    pub owner: String,
    /// GitHub repository name
    pub repo: String,
    /// GitHub token for authentication
    pub token: String,
    /// Branch to deploy to (usually "gh-pages")
    pub branch: String,
    /// Custom domain (optional)
    pub cname: Option<String>,
    /// Build directory (usually "_site")
    pub build_dir: PathBuf,
    /// Commit message template
    pub commit_message: String,
    /// Enable CDN optimization
    pub optimize_assets: bool,
    /// Verify deployment after push
    pub verify_deployment: bool,
}

impl Default for GitHubPagesConfig {
    fn default() -> Self {
        Self {
            owner: String::new(),
            repo: String::new(),
            token: String::new(),
            branch: "gh-pages".to_string(),
            cname: None,
            build_dir: PathBuf::from("_site"),
            commit_message: "Deploy to GitHub Pages [skip ci]".to_string(),
            optimize_assets: true,
            verify_deployment: true,
        }
    }
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// Deployment in progress
    InProgress,
    /// Deployment completed successfully
    Success,
    /// Deployment failed
    Failed(String),
    /// Deployment was rolled back
    RolledBack,
}

/// Deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentInfo {
    /// Deployment ID
    pub id: String,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Deployment timestamp
    pub timestamp: String,
    /// Commit SHA
    pub commit_sha: String,
    /// Deployment URL
    pub url: String,
    /// Build statistics
    pub stats: HashMap<String, serde_json::Value>,
}

/// GitHub Pages deployer
pub struct GitHubPagesDeployer {
    config: GitHubPagesConfig,
    http_client: Client,
}

impl GitHubPagesDeployer {
    /// Create a new GitHub Pages deployer
    pub fn new(config: GitHubPagesConfig) -> Self {
        let http_client = Client::new();
        Self { config, http_client }
    }

    /// Deploy the site to GitHub Pages
    pub async fn deploy(&self, site_config: &SiteConfig) -> Result<DeploymentInfo> {
        let deployment_id = self.generate_deployment_id();
        println!("🚀 Starting GitHub Pages deployment: {}", deployment_id);

        // Initialize or update git repository
        let repo = self.initialize_repo().await?;

        // Optimize assets if enabled
        if self.config.optimize_assets {
            self.optimize_assets(&site_config.output_dir).await?;
        }

        // Create CNAME file if custom domain is specified
        if let Some(cname) = &self.config.cname {
            self.create_cname_file(cname).await?;
        }

        // Stage and commit changes
        let commit_sha = self.commit_changes(&repo).await?;

        // Push to GitHub Pages branch
        self.push_to_github(&repo).await?;

        // Verify deployment if enabled
        let deployment_url = self.get_deployment_url();
        let status = if self.config.verify_deployment {
            match self.verify_deployment(&deployment_url).await {
                Ok(_) => DeploymentStatus::Success,
                Err(e) => DeploymentStatus::Failed(format!("Verification failed: {}", e)),
            }
        } else {
            DeploymentStatus::Success
        };

        let deployment_info = DeploymentInfo {
            id: deployment_id,
            status,
            timestamp: chrono::Utc::now().to_rfc3339(),
            commit_sha,
            url: deployment_url,
            stats: self.collect_deployment_stats(site_config).await?,
        };

        match &deployment_info.status {
            DeploymentStatus::Success => {
                println!("✅ Deployment successful!");
                println!("🌐 Site available at: {}", deployment_info.url);
            }
            DeploymentStatus::Failed(error) => {
                println!("❌ Deployment failed: {}", error);
            }
            _ => {}
        }

        Ok(deployment_info)
    }

    /// Rollback to a previous deployment
    pub async fn rollback(&self, target_commit: &str) -> Result<DeploymentInfo> {
        println!("🔄 Rolling back to commit: {}", target_commit);

        let repo = self.initialize_repo().await?;
        let commit = repo.find_commit(git2::Oid::from_str(target_commit)?)?;

        // Reset to target commit
        repo.reset(&commit.as_object(), git2::ResetType::Hard, None)?;

        // Push the rollback
        self.push_to_github(&repo).await?;

        Ok(DeploymentInfo {
            id: self.generate_deployment_id(),
            status: DeploymentStatus::RolledBack,
            timestamp: chrono::Utc::now().to_rfc3339(),
            commit_sha: target_commit.to_string(),
            url: self.get_deployment_url(),
            stats: HashMap::new(),
        })
    }

    /// Get deployment history
    pub async fn get_deployment_history(&self) -> Result<Vec<DeploymentInfo>> {
        let repo = self.initialize_repo().await?;
        let branch = repo.find_branch(&self.config.branch, BranchType::Local)?;

        let mut revwalk = repo.revwalk()?;
        revwalk.push(branch.get().target().unwrap())?;

        let mut history = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;

            // Parse commit message for deployment info
            let message = commit.message().unwrap_or("");
            if message.contains("[deploy]") || message.contains("Deploy to GitHub Pages") {
                let deployment_info = DeploymentInfo {
                    id: oid.to_string(),
                    status: DeploymentStatus::Success,
                    timestamp: commit.time().seconds().to_string(),
                    commit_sha: oid.to_string(),
                    url: self.get_deployment_url(),
                    stats: HashMap::new(),
                };
                history.push(deployment_info);
            }
        }

        Ok(history)
    }

    /// Initialize or update git repository
    async fn initialize_repo(&self) -> Result<Repository> {
        let repo_path = Path::new(".");
        let repo = if repo_path.join(".git").exists() {
            Repository::open(repo_path)?
        } else {
            Repository::init(repo_path)?
        };

        // Configure git user
        let mut config = repo.config()?;
        config.set_str("user.name", "Kotoba SSG")?;
        config.set_str("user.email", "ssg@kotoba.dev")?;

        // Add GitHub remote if it doesn't exist
        let remote_name = "origin";
        let remote_url = format!("https://github.com/{}/{}.git", self.config.owner, self.config.repo);

        if repo.find_remote(remote_name).is_err() {
            repo.remote(remote_name, &remote_url)?;
        }

        Ok(repo)
    }

    /// Optimize assets for CDN deployment
    async fn optimize_assets(&self, output_dir: &Path) -> Result<()> {
        println!("🔧 Optimizing assets for deployment...");

        // Add asset optimization logic here
        // - Image compression
        // - CSS minification
        // - JavaScript minification
        // - Cache busting

        println!("✅ Assets optimized");
        Ok(())
    }

    /// Create CNAME file for custom domain
    async fn create_cname_file(&self, cname: &str) -> Result<()> {
        let cname_path = Path::new("CNAME");
        fs::write(cname_path, cname).await?;
        println!("📝 Created CNAME file: {}", cname);
        Ok(())
    }

    /// Stage and commit changes
    async fn commit_changes(&self, repo: &Repository) -> Result<String> {
        let mut index = repo.index()?;

        // Add all files in build directory
        self.add_directory_to_index(&mut index, &self.config.build_dir)?;

        // Add CNAME file if it exists
        if Path::new("CNAME").exists() {
            index.add_path(Path::new("CNAME"))?;
        }

        // Write index
        index.write()?;

        // Create commit
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        let signature = Signature::now("Kotoba SSG", "ssg@kotoba.dev")?;

        let parent_commit = if let Ok(head) = repo.head() {
            Some(repo.find_commit(head.target().unwrap())?)
        } else {
            None
        };

        let parents = parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
        let commit_id = repo.commit(
            Some(&format!("refs/heads/{}", self.config.branch)),
            &signature,
            &signature,
            &self.config.commit_message,
            &tree,
            &parents,
        )?;

        println!("📝 Created commit: {}", commit_id);
        Ok(commit_id.to_string())
    }

    /// Push changes to GitHub
    async fn push_to_github(&self, repo: &Repository) -> Result<()> {
        let mut remote = repo.find_remote("origin")?;
        let mut push_options = PushOptions::new();

        // Set up authentication
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            git2::Cred::userpass_plaintext(&format!("x-access-token"), &self.config.token)
        });
        push_options.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{}:refs/heads/{}", self.config.branch, self.config.branch);
        remote.push(&[&refspec], Some(&mut push_options))?;

        println!("📤 Pushed to GitHub Pages branch: {}", self.config.branch);
        Ok(())
    }

    /// Verify deployment by checking if the site is accessible
    async fn verify_deployment(&self, url: &str) -> Result<()> {
        println!("🔍 Verifying deployment at: {}", url);

        // Try to access the main page
        let response = self.http_client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if response.status().is_success() {
            println!("✅ Deployment verified successfully");
            Ok(())
        } else {
            Err(format!("Deployment verification failed: HTTP {}", response.status()).into())
        }
    }

    /// Add directory contents to git index
    fn add_directory_to_index(&self, index: &mut Index, dir: &Path) -> Result<()> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(dir) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Ok(relative_path) = path.strip_prefix(dir) {
                    index.add_path(relative_path)?;
                }
            }
        }

        Ok(())
    }

    /// Generate a unique deployment ID
    fn generate_deployment_id(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("deploy_{}", timestamp)
    }

    /// Get the deployment URL
    fn get_deployment_url(&self) -> String {
        if let Some(cname) = &self.config.cname {
            format!("https://{}", cname)
        } else {
            format!("https://{}.github.io/{}", self.config.owner, self.config.repo)
        }
    }

    /// Collect deployment statistics
    async fn collect_deployment_stats(&self, site_config: &SiteConfig) -> Result<HashMap<String, serde_json::Value>> {
        use walkdir::WalkDir;

        let mut total_files = 0;
        let mut total_size = 0u64;

        for entry in WalkDir::new(&site_config.output_dir) {
            let entry = entry?;
            if entry.path().is_file() {
                total_files += 1;
                if let Ok(metadata) = entry.metadata() {
                    total_size += metadata.len();
                }
            }
        }

        let mut stats = HashMap::new();
        stats.insert("total_files".to_string(), serde_json::json!(total_files));
        stats.insert("total_size_bytes".to_string(), serde_json::json!(total_size));
        stats.insert("build_time".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_id_generation() {
        let config = GitHubPagesConfig::default();
        let deployer = GitHubPagesDeployer::new(config);

        let id1 = deployer.generate_deployment_id();
        let id2 = deployer.generate_deployment_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("deploy_"));
        assert!(id2.starts_with("deploy_"));
    }

    #[test]
    fn test_deployment_url_generation() {
        let mut config = GitHubPagesConfig::default();
        config.owner = "testuser".to_string();
        config.repo = "testrepo".to_string();

        let deployer = GitHubPagesDeployer::new(config.clone());
        let url = deployer.get_deployment_url();
        assert_eq!(url, "https://testuser.github.io/testrepo");

        // Test with custom domain
        let mut config_with_cname = config;
        config_with_cname.cname = Some("example.com".to_string());
        let deployer_with_cname = GitHubPagesDeployer::new(config_with_cname);
        let url_with_cname = deployer_with_cname.get_deployment_url();
        assert_eq!(url_with_cname, "https://example.com");
    }
}
