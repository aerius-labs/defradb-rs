use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use log::{info, error};



const DEFAULT_API_EMAIL: &str = "example@example.com";
const ROOTDIR_KEY: &str = "rootdircli";
const DEFRA_ENV_PREFIX: &str = "DEFRA";
const LOG_LEVEL_DEBUG: &str = "debug";
const LOG_LEVEL_INFO: &str = "info";
const LOG_LEVEL_ERROR: &str = "error";
const LOG_LEVEL_FATAL: &str = "fatal";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    datastore: DatastoreConfig,
    api: APIConfig,
    net: NetConfig,
    log: LoggingConfig,
    rootdir: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct DatastoreConfig {
    store: String,
    memory: MemoryConfig,
    badger: BadgerConfig,
    max_txn_retries: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct BadgerConfig {
    path: PathBuf,
    value_log_file_size: ByteSize,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemoryConfig {
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct APIConfig {
    address: String,
    tls: bool,
    allowed_origins: Vec<String>,
    pub_key_path: PathBuf,
    priv_key_path: PathBuf,
    email: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NetConfig {
    p2p_address: String,
    p2p_disabled: bool,
    peers: String,
    pub_sub_enabled: bool,
    relay_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoggingConfig {
    level: String,
    stacktrace: bool,
    format: String,
    output: String,
    caller: bool,
    no_color: bool,
    logger: String,
    named_overrides: HashMap<String, NamedLoggingConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NamedLoggingConfig {
    name: String,
    logging_config: LoggingConfig,
}

