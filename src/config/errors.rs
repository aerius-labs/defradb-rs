use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to write file: {0}")]
    FailedToWriteFile(String),

    #[error("failed to remove config file")]
    FailedToRemoveConfigFile,

    #[error("path cannot be just ~ (home directory)")]
    PathCannotBeHomeDir,

    #[error("unable to expand home directory")]
    UnableToExpandHomeDir,

    #[error("no database URL provided")]
    NoDatabaseURLProvided,

    #[error("invalid database URL")]
    InvalidDatabaseURL,

    #[error("could not get logging config")]
    LoggingConfigNotObtained,

    #[error("failed to validate config")]
    FailedToValidateConfig,

    #[error("invalid RPC timeout: {0}")]
    InvalidRPCTimeout(String),

    #[error("invalid RPC MaxConnectionIdle: {0}")]
    InvalidRPCMaxConnectionIdle(String),

    #[error("invalid P2P address: {0}, {1}")]
    InvalidP2PAddress(String, String),

    #[error("invalid RPC address: {0}")]
    InvalidRPCAddress(String),

    #[error("invalid bootstrap peers: {0}, {1}")]
    InvalidBootstrapPeers(String, String),

    #[error("invalid log level: {0}")]
    InvalidLogLevel(String),

    #[error("invalid store type: {0}")]
    InvalidDatastoreType(String),

    #[error("invalid override config for {0}")]
    OverrideConfigConvertFailed(String),

    #[error("invalid log format: {0}")]
    InvalidLogFormat(String),

    #[error("failed to marshal Config to JSON")]
    ConfigToJSONFailed,

    #[error("invalid named logger name: {0}")]
    InvalidNamedLoggerName(String),

    #[error("could not process config template")]
    ConfigTemplateFailed,

    #[error("could not get named logger config: {0}")]
    CouldNotObtainLoggerConfig(String, String),

    #[error("logging config parameter was not provided as <key>=<value> pair: {0}")]
    NotProvidedAsKV(String),

    #[error("could not parse type: {0}")]
    CouldNotParseType(String),

    #[error("unknown logger parameter: {0}")]
    UnknownLoggerParameter(String),

    #[error("invalid logger name: {0}")]
    InvalidLoggerName(String),

    #[error("duplicate logger name: {0}")]
    DuplicateLoggerName(String),

    #[error("failed to read config")]
    ReadingConfigFile,

    #[error("failed to load config")]
    LoadingConfig,

    #[error("unable to parse byte size")]
    UnableToParseByteSize,

    #[error("invalid logger config: {0}")]
    InvalidLoggerConfig(String),

    #[error("invalid datastore path: {0}")]
    InvalidDatastorePath(String),

    #[error("missing port number")]
    MissingPortNumber,

    #[error("cannot provide port with domain name")]
    NoPortWithDomain,

    #[error("invalid root directory: {0}")]
    InvalidRootDir(String),

    #[error("custom error: {0}")]
    Custom(String),
}
