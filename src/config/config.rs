use std::collections::{HashMap, HashSet};
use std::fmt::Error;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use log::{info, error};
use config::{File, Environment, FileFormat, Value};
use multiaddr::{Multiaddr};
use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::config::config_file::DEFAULT_CONFIG_TEMPLATE;

use crate::config::config_utils::{ByteSize, expand_home_dir};
use crate::config::errors::ConfigError;


const DEFAULT_API_EMAIL: &str = "example@example.com";
const ROOTDIR_KEY: &str = "rootdircli";
const DEFRA_ENV_PREFIX: &str = "DEFRA";
const LOG_LEVEL_DEBUG: &str = "debug";
const LOG_LEVEL_INFO: &str = "info";
const LOG_LEVEL_ERROR: &str = "error";
const LOG_LEVEL_FATAL: &str = "fatal";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub datastore: DatastoreConfig,
    pub api: APIConfig,
    pub net: NetConfig,
    pub log: LoggingConfig,
    pub rootdir: String,

    #[serde(skip)]
    pub config: config::Config,
}

impl Config {
    pub fn default_config() -> Result<Self, ConfigError> {
        let mut config = config::Config::default();

        // TODO: add default config
        // config.set_default("Datastore", DatastoreConfig::default_data_store_config())?;

        config.set_default("API", json!(APIConfig::default_api_config()).as_str().unwrap().to_string())
            .map_err(|e| ConfigError::Custom(format!("Failed to set default api config: {}", e)))?;

        config.set_default("Net", json!(NetConfig::default_net_config()).as_str().unwrap().to_string())
            .map_err(|e| ConfigError::Custom(format!("Failed to set default net config: {}", e)))?;

        config.set_default("Log", json!(LoggingConfig::default_log_config()).as_str().unwrap().to_string())
            .map_err(|e| ConfigError::Custom(format!("Failed to set default log config: {}", e)))?;

        config.set_default("Rootdir", "".to_string())
            .map_err(|e| ConfigError::Custom(format!("Failed to set default rootdir: {}", e)))?;

        // TODO: find equivalents fo the same
        // config.set_env_prefix("defra_env_prefix");
        // config.set_env_replacer("_", ".");

        config.merge(File::new("DefaultConfigFileName", FileFormat::Toml)).map_err(|e| ConfigError::Custom(format!("Failed to merge default config file: {}", e)))?;

        let cfg = Config {
            datastore: config.get("Datastore").map_err(|e| ConfigError::Custom(format!("Failed to get datastore: {}", e)))?,
            api: config.get("API").map_err(|e| ConfigError::Custom(format!("Failed to get api: {}", e)))?,
            net: config.get("Net").map_err(|e| ConfigError::Custom(format!("Failed to get net: {}", e)))?,
            log: config.get("Log").map_err(|e| ConfigError::Custom(format!("Failed to get log: {}", e)))?,
            rootdir: config.get("Rootdir").map_err(|e| ConfigError::Custom(format!("Failed to get rootdir: {}", e)))?,
            config,
        };

        Ok(cfg)
    }

    pub fn load_with_rootdir(&mut self, with_rootdir: bool) -> Result<(), ConfigError> {
        if with_rootdir {
            self.config.merge(File::with_name(self.rootdir.as_str())).map_err(|e| ConfigError::Custom(format!("Failed to merge config file: {}", e)))?;
        }

        self.config.clone().try_into::<Self>().map_err(|e| ConfigError::Custom(format!("Failed to load config: {}", e)))?;
        self.validate()?;
        self.params_preprocessing()?;
        self.load()?;

        Ok(())
    }

    fn set_rootdir(&mut self, rootdir: &str) -> Result<(), ConfigError> {
        if rootdir.is_empty() {
            return Err(ConfigError::InvalidRootDir(rootdir.to_string()).into());
        }

        self.rootdir = fs::canonicalize(rootdir).map(|p| p.to_str().unwrap().to_string()).map_err(|e| ConfigError::Custom(format!("Failed to canonicalize rootdir: {}", e)))?;
        self.config.set_default("rootdir", self.rootdir.clone()).map_err(|e| ConfigError::Custom(format!("Failed to set rootdir: {}", e)))?;
        Ok(())
    }

    fn validate(&self) -> Result<(), ConfigError> {
        self.datastore.validate()?;
        self.api.validate()?;
        self.net.validate()?;
        self.log.validate()?;
        Ok(())
    }

    fn params_preprocessing(&mut self) -> Result<(), ConfigError> {
        let mut update_path = |key: &str| {
            let mut path = self.config.get::<String>(key).unwrap_or_default();
            if !Path::new(&path).is_absolute() {
                self.config.set(key, self.rootdir.clone()  + path.as_str()).unwrap();
            }
        };

        update_path("datastore.badger.path");
        update_path("api.privkeypath");
        update_path("api.pubkeypath");

        if let Ok(loglogger_as_string_slice) = self.config.get::<Vec<String>>("log.logger") {
            let combined = loglogger_as_string_slice.join(";");
            self.config.set("log.logger", combined).unwrap();
        }

        // Assuming expand_home_dir exists
        expand_home_dir(&mut self.api.priv_key_path).map_err(|e| ConfigError::Custom(format!("Unable to expand home directory: {}", e)))?;
        expand_home_dir(&mut self.api.pub_key_path).map_err(|e| ConfigError::Custom(format!("Unable to expand home directory: {}", e)))?;

        // Assuming ByteSize and its set() method exist
        let mut bs = ByteSize::default();
        let value = self.config.get::<String>("datastore.badger.valuelogfilesize").unwrap_or_default();
        bs.set(&value)?;
        self.datastore.badger.value_log_file_size = bs;

        Ok(())
    }

    fn load(&mut self) -> Result<(), ConfigError> {
        self.log.load()?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ConfigError> {
        let mut handlebars = Handlebars::new();
        let config_template = DEFAULT_CONFIG_TEMPLATE;
        handlebars.register_template_string("configTemplate", config_template).map_err(|e| ConfigError::Custom(format!("Could not register config template: {}", e)))?;

        let rendered = handlebars.render("configTemplate", &self).map_err(|e| ConfigError::Custom(format!("Could not process config template: {}", e)))?;

        Ok(rendered.into_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatastoreConfig {
    store: String,
    memory: MemoryConfig,
    badger: BadgerConfig,
    max_txn_retries: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BadgerConfig {
    path: String,
    value_log_file_size: ByteSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemoryConfig {
    size: u64,
}

impl DatastoreConfig {
    // TODO: add default config

    fn validate(&self) -> Result<(), ConfigError> {
        match self.store.as_str() {
            "badger" | "memory" => Ok(()),
            _ => Err(ConfigError::InvalidDatastoreType(self.store.clone())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct APIConfig {
    address: String,
    tls: bool,
    allowed_origins: Vec<String>,
    pub_key_path: String,
    priv_key_path: String,
    email: String,
}


impl APIConfig {
    fn default_api_config() -> Self {
        APIConfig {
            address: "localhost:9181".to_string(),
            tls: false,
            allowed_origins: vec![],
            pub_key_path: "certs/server.key".to_string(),
            priv_key_path: "certs/server.crt".to_string(),
            email: DEFAULT_API_EMAIL.to_string(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.address.is_empty() {
            return Err(ConfigError::InvalidDatabaseURL);
        }

        if self.address == "localhost" || self.address.parse::<SocketAddr>().is_ok() {
            return Err(ConfigError::MissingPortNumber);
        }

        if Self::is_valid_domain_name(&self.address) {
            return Ok(());
        }

        // Try parsing as "host:port"
        if let Ok(addrs) = (&self.address[..], 0).to_socket_addrs() {
            for addr in addrs {
                if addr.ip().is_loopback() {
                    return Ok(());
                }
                if !Self::is_valid_domain_name(&addr.ip().to_string()) {
                    return Err(ConfigError::NoPortWithDomain);
                }
            }
        } else {
            return Err(ConfigError::InvalidDatabaseURL);
        }

        Ok(())
    }

    fn is_valid_domain_name(domain: &str) -> bool {
        let config = idna::Config::default()
            .transitional_processing(false)
            .use_std3_ascii_rules(true);

        match idna::Config::to_ascii(config, domain, ) {
            Ok(ascii_domain) => ascii_domain == domain,
            Err(_) => false,
        }
    }

    pub fn address_to_url(&self) -> String {
        if self.tls {
            format!("https://{}", self.address)
        } else {
            format!("http://{}", self.address)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetConfig {
    p2p_address: String,
    p2p_disabled: bool,
    peers: String,
    pub_sub_enabled: bool,
    relay_enabled: bool,
}

impl NetConfig {

    fn default_net_config() -> Self {
        return NetConfig {
            p2p_address: "/ip4/0.0.0.0/tcp/9171".to_string(),
            p2p_disabled: false,
            peers: "".to_string(),
            pub_sub_enabled: true,
            relay_enabled: false,
        }
    }
    fn validate(&self) -> Result<(), ConfigError> {
        self.p2p_address.parse::<Multiaddr>().map_err(|err| ConfigError::InvalidP2PAddress(err.to_string(), self.p2p_address.clone()))?;

        if !self.peers.is_empty() {
            let peers: Vec<&str> = self.peers.split(',').collect();
            for addr in &peers {
                addr.parse::<Multiaddr>().map_err(|err| ConfigError::InvalidBootstrapPeers(err.to_string(), peers.clone().iter().map(|x| (**x).to_string()).collect::<Vec<_>>().join(", ")))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NamedLoggingConfig {
    name: String,
    logging_config: LoggingConfig,
}

impl LoggingConfig {
    fn default_log_config() -> Self {
        LoggingConfig {
            level: LOG_LEVEL_INFO.to_string(),
            stacktrace: false,
            format: "csv".to_string(),
            output: "stderr".to_string(),
            caller: false,
            no_color: false,
            logger: "".to_string(),
            named_overrides: HashMap::new(),
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        fn valid_level(level: &str) -> bool {
            match level {
                LOG_LEVEL_DEBUG | LOG_LEVEL_INFO | LOG_LEVEL_ERROR | LOG_LEVEL_FATAL => true,
                _ => false,
            }
        }

        fn ensure_unique_keys(kvs: &Vec<HashMap<&str, &str>>) -> Result<(), ConfigError> {
            let mut keys = HashSet::new();
            for kv in kvs {
                for k in kv.keys() {
                    if keys.contains(k) {
                        return Err(ConfigError::DuplicateLoggerName(k.to_string()))
                    }
                    keys.insert(k);
                }
            }
            Ok(())
        }

        let valid_levels = ["logLevelDebug", "logLevelInfo", "logLevelError", "logLevelFatal"];

        let parts: Vec<&str> = self.level.split(',').collect();

        if !parts.is_empty() && !valid_levels.contains(&parts[0]) {
            return Err(ConfigError::InvalidLogLevel(parts[0].to_string()));
        }

        let mut kvs: Vec<HashMap<&str, &str>> = Vec::new();
        for kv in &parts[1..] {
            let parsed_kv: Vec<&str> = kv.split('=').collect();
            if parsed_kv.len() != 2 || parsed_kv[0].is_empty() || parsed_kv[1].is_empty() {
                return Err(ConfigError::NotProvidedAsKV(kv.to_string()));
            }

            let mut new_kv = HashMap::new();
            new_kv.insert(parsed_kv[0], parsed_kv[1]);
            kvs.push(new_kv);
        }

        if !self.logger.is_empty() {
            let named_configs: Vec<&str> = self.logger.split(';').collect();
            for config in &named_configs {
                let parts: Vec<&str> = config.split(',').collect();
                if parts.len() < 2 {
                    return Err(ConfigError::InvalidLoggerConfig("unexpected format (expected: `module,key=value;module,key=value;...`".to_string()).into());
                }
                if parts[0].is_empty() {
                    return Err(ConfigError::InvalidLoggerName("".to_string()).into());
                }
                for pair in &parts[1..] {
                    let parsed_kv: Vec<&str> = pair.split('=').collect();
                    if parsed_kv.len() != 2 || parsed_kv[0].is_empty() || parsed_kv[1].is_empty() {
                        return Err(ConfigError::NotProvidedAsKV(pair.to_string()).into());
                    }
                    match parsed_kv[0] {
                        "format" | "output" | "nocolor" | "stacktrace" | "caller" => {}
                        "level" if valid_levels.contains(&parsed_kv[1]) => {}
                        _ => return Err(ConfigError::UnknownLoggerParameter(parsed_kv[0].to_string()).into()),
                    }
                }
            }
        }
        Ok(())
    }

    fn load(&mut self) -> Result<(), ConfigError> {
        // load loglevel
        let parts_copy = self.level.clone();
        let parts: Vec<&str> = parts_copy.split(',').collect();
        if !parts.is_empty() {
            self.level = parts[0].to_string();
        }
        if parts.len() > 1 {
            for kv in &parts[1..] {
                let parsed_kv: Vec<&str> = kv.split('=').collect();
                if parsed_kv.len() != 2 {
                    return Err(ConfigError::InvalidLogLevel(kv.to_string()).into());
                }
                match self.get_or_create_named_logger(parsed_kv[0]) {
                    Ok(c) => c.logging_config.level = parsed_kv[1].to_string(),
                    Err(e) => return Err(ConfigError::CouldNotObtainLoggerConfig(e.to_string(), parsed_kv[0].to_string()).into()),
                }
            }
        }

        // load logger
        if !self.logger.is_empty() {
            let logger_copy = self.logger.clone();
            let s: Vec<&str> = logger_copy.split(';').collect();
            for v in s {
                let vs: Vec<&str> = v.split(',').collect();
                let mut override_logger = self.get_or_create_named_logger(vs[0])?;
                override_logger.name = vs[0].to_string();
                for v in &vs[1..] {
                    let parsed_kv: Vec<&str> = v.split('=').collect();
                    if parsed_kv.len() != 2 {
                        return Err(ConfigError::NotProvidedAsKV(v.to_string()).into());
                    }
                    match parsed_kv[0].to_lowercase().as_str() {
                        "level" => override_logger.logging_config.level = parsed_kv[1].to_string(),
                        "format" => override_logger.logging_config.format = parsed_kv[1].to_string(),
                        "output" => override_logger.logging_config.output = parsed_kv[1].to_string(),
                        "stacktrace" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.stacktrace = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string()).into()),
                        },
                        "nocolor" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.no_color = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string()).into()),
                        },
                        "caller" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.caller = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string()).into()),
                        },
                        _ => return Err(ConfigError::UnknownLoggerParameter(parsed_kv[0].to_string()).into()),
                    }
                }
            }
        }

        // TODO: Implmenet corresponding to_logger_config() method
        // let c = self.to_logger_config()?;

        // TODO: set logging config
        // logging::set_config(c);
        Ok(())
    }

    fn get_or_create_named_logger(&mut self, name: &str) -> Result<&mut NamedLoggingConfig, ConfigError> {
        // Check if the named logger exists.
        if !self.named_overrides.contains_key(name) {
            // If doesn't exist, create a new named logger
            let named_cfg = NamedLoggingConfig {
                name: name.to_string(),
                logging_config: self.clone(),
            };
            self.named_overrides.insert(name.to_string(), named_cfg);
        }

        // At this point, either the named logger existed or we created it. Return it.
        Ok(self.named_overrides.get_mut(name).unwrap())
    }
}

impl NamedLoggingConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        self.logging_config.validate()
    }
}




