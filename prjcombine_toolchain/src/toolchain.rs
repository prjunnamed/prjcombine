use serde::Deserialize;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::Path;
use std::process::Command;
use which::which_in;

#[derive(Debug, Clone, Deserialize)]
pub struct Toolchain {
    pub use_wine: bool,
    pub env: HashMap<String, String>,
}

impl Toolchain {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let s = read_to_string(path)?;
        Ok(toml::from_str(&s).unwrap())
    }

    pub fn command(&self, cmd: &str) -> Command {
        let mut res: Command;
        if self.use_wine {
            res = Command::new("wine");
            res.arg(cmd);
        } else if let Some(path) = self.env.get("PATH") {
            let rcmd = which_in(cmd, Some(path), "/").unwrap();
            res = Command::new(rcmd);
        } else {
            res = Command::new(cmd);
        }
        for (k, v) in self.env.iter() {
            res.env(k, v);
        }
        res
    }
}
