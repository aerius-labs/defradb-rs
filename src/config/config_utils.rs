use std::path::{Path, PathBuf};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::config::ConfigError;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ByteSize(u64);

const B: ByteSize = ByteSize(1);
const KiB: ByteSize = ByteSize(B.0 << 10);
const MiB: ByteSize = ByteSize(KiB.0 << 10);
const GiB: ByteSize = ByteSize(MiB.0 << 10);
const TiB: ByteSize = ByteSize(GiB.0 << 10);
const PiB: ByteSize = ByteSize(TiB.0 << 10);

impl ByteSize {
    pub fn set(&mut self, s: &str) -> Result<(), ConfigError> {
        let (digit_string, unit): (String, String) = s.chars().partition(|&c| c.is_digit(10));

        let digits = digit_string.parse::<u64>().map_err(|_| ConfigError::Custom(s.to_string()))?;

        match unit.to_uppercase().trim() {
            "B" => *self = ByteSize(digits * B.0),
            "KB" | "KIB" => *self = ByteSize(digits * KiB.0),
            "MB" | "MIB" => *self = ByteSize(digits * MiB.0),
            "GB" | "GIB" => *self = ByteSize(digits * GiB.0),
            "TB" | "TIB" => *self = ByteSize(digits * TiB.0),
            "PB" | "PIB" => *self = ByteSize(digits * PiB.0),
            _ => *self = ByteSize(digits),
        }

        Ok(())
    }

    pub fn to_string(&self) -> String {
        const UNIT: u64 = 1024;
        let mut div = UNIT;
        let mut exp = 0;
        let mut n = self.0 / UNIT;

        while n >= UNIT {
            div *= UNIT;
            exp += 1;
            n /= UNIT;
        }

        format!("{} {}iB", self.0 / div, "KMGTP".chars().nth(exp).unwrap_or(' '))
    }
}

impl FromStr for ByteSize {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, ConfigError> {
        let mut byte_size = ByteSize(0);
        byte_size.set(s)?;
        Ok(byte_size)
    }
}

pub fn expand_home_dir(path: &str) -> Result<PathBuf, String> {
    if path == "~" {
        return Err("Path cannot be home directory.".to_string());
    } else if path.starts_with("~/") {
        let home_dir = dirs::home_dir().ok_or("Unable to get home directory.".to_string())?;
        return Ok(home_dir.join(&path[2..]));
    }

    Ok(Path::new(path).to_path_buf())
}

fn is_lowercase_alpha(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_lowercase())
}

fn parse_kv(kv: &str) -> Result<(String, String), String> {
    let mut parts = kv.splitn(2, '=');
    let key = parts.next().unwrap_or_default().to_string();
    let value = parts.next().unwrap_or_default().to_string();

    if key.is_empty() || value.is_empty() {
        return Err(format!("Invalid key-value pair: {}", kv));
    }

    Ok((key, value))
}