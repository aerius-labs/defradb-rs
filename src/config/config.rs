use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Error;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::{Path, PathBuf};
use log::{info, error};
use config::{File, Environment, FileFormat};
use multiaddr::{Multiaddr};

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
struct Config {
    datastore: DatastoreConfig,
    api: APIConfig,
    net: NetConfig,
    log: LoggingConfig,
    rootdir: String,
    config: config::Config,
}

impl Config {
    pub fn default_config() -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = config::Config::default();

        config.set_default("Datastore", DatastoreConfig::default_data_store_config())?;
        config.set_default("API", APIConfig::default_api_config())?;
        config.set_default("Net", NetConfig::default_net_config())?;
        config.set_default("Log", LoggingConfig::default_log_config())?;
        config.set_default("Rootdir", PathBuf::new())?;

        config.set_env_prefix("defra_env_prefix");
        config.set_env_replacer("_", ".");

        config.merge(File::new("DefaultConfigFileName", FileFormat::Toml))?;

        let cfg = Config {
            datastore: config.get("Datastore")?,
            api: config.get("API")?,
            net: config.get("Net")?,
            log: config.get("Log")?,
            rootdir: config.get("Rootdir")?,
            config,
        };

        Ok(cfg)
    }

    pub fn load_with_rootdir(&mut self, with_rootdir: bool) -> Result<(), Box<dyn std::error::Error>> {
        if with_rootdir {
            self.config.merge(File::with_name(self.rootdir.to_str().unwrap()))?;
        }

        self.config.try_into::<Self>()?;
        self.validate()?;
        self.params_preprocessing()?;
        self.load()?;

        Ok(())
    }

    fn set_rootdir(&mut self, rootdir: &str) -> Result<(), ConfigError> {
        if rootdir.is_empty() {
            return Err(ConfigError::InvalidRootDir(rootdir.to_string()));
        }

        self.rootdir = fs::canonicalize(rootdir).into()?; // This gets the absolute path
        self.config.set_default("rootdir", &self.rootdir.display().to_string())?;
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
        let update_path = |key: &str| {
            let path = self.config.get::<String>(key).unwrap_or_default();
            if !Path::new(&path).is_absolute() {
                self.config.set(key, self.rootdir.join(path).display().to_string()).unwrap();
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
        expand_home_dir(&mut self.api.priv_key_path)?;
        expand_home_dir(&mut self.api.pub_key_path)?;

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

impl DatastoreConfig {
    // TODO: add default config

    fn validate(&self) -> Result<(), ConfigError> {
        match self.store.as_str() {
            "badger" | "memory" => Ok(()),
            _ => Err(ConfigError::InvalidDatastoreType(self.store.clone())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
    fn validate(&self) -> Result<(), Error> {
        self.p2p_address.parse::<Multiaddr>().map_err(|err| ConfigError::InvalidP2PAddress(err.to_string(), self.p2p_address.clone()).into())?;

        if !self.peers.is_empty() {
            let peers: Vec<&str> = self.peers.split(',').collect();
            for addr in peers {
                self.p2p_address.parse::<Multiaddr>().map_err(|err| ConfigError::InvalidBootstrapPeers(err.to_string(), peers.into_iter().map(|x| x.to_string()).collect()).into())?;
            }
        }

        Ok(())
    }
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

    fn validate(&self) -> Result<(), Error> {
        fn valid_level(level: &str) -> bool {
            match level {
                LOG_LEVEL_DEBUG | LOG_LEVEL_INFO | LOG_LEVEL_ERROR | LOG_LEVEL_FATAL => true,
                _ => false,
            }
        }

        fn ensure_unique_keys(kvs: &Vec<HashMap<&str, &str>>) -> Result<(), Error> {
            let mut keys = HashSet::new();
            for kv in kvs {
                for k in kv.keys() {
                    if keys.contains(k) {
                        return ConfigError::DuplicateLoggerName(k.to_string()).into()
                    }
                    keys.insert(k);
                }
            }
            Ok(())
        }

        let valid_levels = ["logLevelDebug", "logLevelInfo", "logLevelError", "logLevelFatal"];

        let parts: Vec<&str> = self.level.split(',').collect();

        if !parts.is_empty() && !valid_levels.contains(&parts[0]) {
            return ConfigError::InvalidLogLevel(parts[0].to_string()).into();
        }

        let mut kvs: Vec<HashMap<&str, &str>> = Vec::new();
        for kv in &parts[1..] {
            let parsed_kv: Vec<&str> = kv.split('=').collect();
            if parsed_kv.len() != 2 || parsed_kv[0].is_empty() || parsed_kv[1].is_empty() {
                return ConfigError::NotProvidedAsKV(kv.to_string()).into();
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
                    return ConfigError::InvalidLoggerConfig("unexpected format (expected: `module,key=value;module,key=value;...`".to_string()).into();
                }
                if parts[0].is_empty() {
                    return ConfigError::InvalidLoggerName("".to_string()).into();
                }
                for pair in &parts[1..] {
                    let parsed_kv: Vec<&str> = pair.split('=').collect();
                    if parsed_kv.len() != 2 || parsed_kv[0].is_empty() || parsed_kv[1].is_empty() {
                        return ConfigError::NotProvidedAsKV(pair.to_string()).into();
                    }
                    match parsed_kv[0] {
                        "format" | "output" | "nocolor" | "stacktrace" | "caller" => {}
                        "level" if valid_levels.contains(&parsed_kv[1]) => {}
                        _ => return ConfigError::UnknownLoggerParameter(parsed_kv[0].to_string()).into(),
                    }
                }
            }
        }
        Ok(())
    }

    fn load(&mut self) -> Result<(), ConfigError> {
        // load loglevel
        let parts: Vec<&str> = self.level.split(',').collect();
        if !parts.is_empty() {
            self.level = parts[0].to_string();
        }
        if parts.len() > 1 {
            for kv in &parts[1..] {
                let parsed_kv: Vec<&str> = kv.split('=').collect();
                if parsed_kv.len() != 2 {
                    return Err(ConfigError::InvalidLogLevel(kv.to_string()));
                }
                match self.get_or_create_named_logger(parsed_kv[0]) {
                    Ok(c) => c.logging_config.level = parsed_kv[1].to_string(),
                    Err(e) => return Err(ConfigError::CouldNotObtainLoggerConfig(e.to_string(), parsed_kv[0].to_string())),
                }
            }
        }

        // load logger
        if !self.logger.is_empty() {
            let s: Vec<&str> = self.logger.split(';').collect();
            for v in s {
                let vs: Vec<&str> = v.split(',').collect();
                let mut override_logger = self.get_or_create_named_logger(vs[0])?;
                override_logger.name = vs[0].to_string();
                for v in &vs[1..] {
                    let parsed_kv: Vec<&str> = v.split('=').collect();
                    if parsed_kv.len() != 2 {
                        return Err(ConfigError::NotProvidedAsKV(v.to_string()));
                    }
                    match parsed_kv[0].to_lowercase().as_str() {
                        "level" => override_logger.logging_config.level = parsed_kv[1].to_string(),
                        "format" => override_logger.logging_config.format = parsed_kv[1].to_string(),
                        "output" => override_logger.logging_config.output = parsed_kv[1].to_string(),
                        "stacktrace" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.stacktrace = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string())),
                        },
                        "nocolor" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.no_color = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string())),
                        },
                        "caller" => match parsed_kv[1].parse::<bool>() {
                            Ok(val) => override_logger.logging_config.caller = val,
                            Err(_) => return Err(ConfigError::CouldNotParseType("bool".to_string())),
                        },
                        _ => return Err(ConfigError::UnknownLoggerParameter(parsed_kv[0].to_string())),
                    }
                }
            }
        }

        let c = self.to_logger_config()?;

        // TODO: set logging config
        // logging::set_config(c);
        Ok(())
    }

    fn get_or_create_named_logger(&mut self, name: &str) -> Result<&mut NamedLoggingConfig, Error> {
        if let Some(named_cfg) = self.named_overrides.get_mut(name) {
            return Ok(named_cfg);
        }

        // If doesn't exist, create a new named logger
        let named_cfg = NamedLoggingConfig {
            name: name.to_string(),
            logging_config: self.clone(),
        };
        self.named_overrides.insert(name.to_string(), named_cfg);
        Ok(self.named_overrides.get_mut(name).unwrap())
    }
}

impl NamedLoggingConfig {
    fn validate(&self) -> Result<(), Error> {
        self.logging_config.validate()
    }
}




