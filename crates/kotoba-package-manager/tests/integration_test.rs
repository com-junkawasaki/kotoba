use kotoba_package_manager::{DependencyInfo, PackageManager, ProjectConfig};
use std::collections::HashMap;
use tempfile::tempdir;
use tokio::fs;

#[tokio::test]
async fn test_install_npm_package() -> Result<(), anyhow::Error> {
    // 1. Create a temporary directory for the test project
    let dir = tempdir()?;
    let project_root = dir.path();
    std::env::set_current_dir(project_root)?;

    // 2. Setup a dummy kotoba.toml
    let mut dependencies = HashMap::new();
    dependencies.insert(
        "react".to_string(),
        DependencyInfo {
            version: "18.2.0".to_string(), // Pin version for deterministic test
            source: None, // Assuming npm is the default
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

    // 3. Run the install command
    let pm = PackageManager::new().await?;
    pm.install().await?;

    // 4. Verify that the package was installed correctly
    let package_json_path = project_root.join("node_modules/react/package.json");
    assert!(package_json_path.exists(), "react/package.json should exist");

    let package_json_content = fs::read_to_string(package_json_path).await?;
    let package_json: serde_json::Value = serde_json::from_str(&package_json_content)?;

    assert_eq!(package_json["name"], "react");
    assert_eq!(package_json["version"], "18.2.0");

    dir.close()?;
    Ok(())
}
