use std::collections::HashMap;
use std::path::Path;
use std::fs::read;
use std::process::Command;
use crate::error::Error;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Toolchain {
    pub use_wine: bool,
    pub env: HashMap<String, String>,
}

impl Toolchain {
    pub fn from_file<P: AsRef<Path>> (path: P) -> Result<Self, Error> {
        let s = read(path)?;
        Ok(toml::from_slice(&s).unwrap())
    }

    pub fn command(&self, cmd: &str) -> Command {
        let mut res: Command;
        if self.use_wine {
            res = Command::new("wine");
            res.arg(cmd);
        } else {
            res = Command::new(cmd);
        }
        for (k, v) in self.env.iter() {
            res.env(k, v);
        }
        res
    }
}
