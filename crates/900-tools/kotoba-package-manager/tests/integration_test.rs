use kotoba_package_manager::{DependencyInfo, PackageManager, ProjectConfig};
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn test_install_and_verify_npm_package() -> Result<(), anyhow::Error> {
    // 1. Setup temporary project
    let dir = tempdir()?;
    let project_root = dir.path();
    std::env::set_current_dir(project_root)?;

    // 2. Setup kotoba.toml
    let mut dependencies = HashMap::new();
    dependencies.insert(
        "react".to_string(),
        DependencyInfo {
            version: "18.2.0".to_string(),
            source: None,
        },
    );
    let project_config = ProjectConfig {
        name: "test-project".to_string(),
        version: "0.1.0".to_string(),
        description: None,
        dependencies,
        dev_dependencies: HashMap::new(),
        scripts: HashMap::new(),
    };
    let toml_content = toml::to_string(&project_config)?;
    fs::write("kotoba.toml", toml_content).await?;

    // 3. First install
    let pm = PackageManager::new().await?;
    pm.install().await?;

    // 4. Verify initial installation
    let package_json_path = project_root.join("node_modules/react/package.json");
    assert!(package_json_path.exists(), "react/package.json should exist after first install");
    let lockfile_path = project_root.join("kotoba.lock");
    assert!(lockfile_path.exists(), "kotoba.lock should exist after first install");

    // 5. Corrupt the installation by deleting node_modules
    fs::remove_dir_all(project_root.join("node_modules")).await?;
    
    println!("Simulating corruption by deleting node_modules.");

    // 6. Run install again to trigger verification and re-installation
    // The current implementation doesn't verify node_modules existence, it only checks the cache.
    // To make this test pass, the installer logic needs to be aware of the lockfile and node_modules state.
    // Let's adjust the installer to handle this.
    // For now, this test will likely fail as the logic assumes cache hit means installed.
    // We'll proceed to demonstrate the current state.
    pm.install().await?;
    
    // 7. Verify again
    assert!(package_json_path.exists(), "react/package.json should exist after re-install");

    dir.close()?;
    Ok(())
}
