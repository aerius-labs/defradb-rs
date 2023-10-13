use std::fs;
use std::path::{Path, PathBuf};
use std::io;
use handlebars::Handlebars;
use once_cell::sync::Lazy;
use std::fs::Permissions;
use std::os::unix::fs::PermissionsExt;

use super::Config;
use super::ConfigError;

const DEFAULT_CONFIG_FILE_NAME: &str = "config.yaml";
static DEFAULT_DIR_PERM: Lazy<Permissions> = Lazy::new(|| Permissions::from_mode(0o700));
static DEFAULT_CONFIG_FILE_PERM: Lazy<Permissions> = Lazy::new(|| Permissions::from_mode(0o644));

// Embed the default config template
pub const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("configfile_yaml.gotmpl");

impl Config {
    pub fn config_file_path(&self) -> String {
        self.rootdir.clone() + DEFAULT_CONFIG_FILE_NAME
    }

    pub fn write_config_file(&self) -> Result<(), ConfigError> {
        let path = self.config_file_path();
        let buffer = self.to_bytes()?;  // to_bytes now returns a Result<String, String>
        fs::write(&path, buffer).map_err(|e| ConfigError::Custom(format!("Failed to write file: {}", e)))?;
        println!("Created config file at {:?}", path);  // Replace with proper logging
        Ok(())
    }

    pub fn delete_config_file(&self) -> Result<(), ConfigError> {
        let path = self.config_file_path();
        fs::remove_file(&path).map_err(|e| ConfigError::Custom(format!("Failed to remove config file: {}", e)))?;
        println!("Deleted config file at {:?}", path);  // Replace with proper logging
        Ok(())
    }

    pub fn create_root_dir_and_config_file(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.rootdir).map_err(|e| ConfigError::Custom(format!("Failed to create root directory: {}", e)))?;
        // TODO: replace with proper logging
        println!("Created root directory at {:?}", self.rootdir);  // Replace with proper logging
        self.write_config_file()
    }

    pub fn config_file_exists(&self) -> bool {
        let path = self.config_file_path();
        match fs::metadata(&path) {
            Ok(metadata) => !metadata.is_dir(),
            Err(_) => false,
        }
    }
}

pub fn default_root_dir() -> PathBuf {
    dirs::home_dir().expect("Failed to get home directory").join(".defradb")
}

pub fn folder_exists(folder_path: &Path) -> bool {
    match fs::metadata(folder_path) {
        Ok(metadata) => metadata.is_dir(),
        Err(_) => false,
    }
}