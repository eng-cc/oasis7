use crate::error::{ContextError, Result};
use std::collections::{BTreeMap, BTreeSet};

pub struct Arguments {
    pub positional: Vec<String>,
    values: BTreeMap<String, Vec<String>>,
    switches: BTreeSet<String>,
}

impl Arguments {
    pub fn parse(args: &[String]) -> Result<Self> {
        let mut parsed = Self {
            positional: vec![],
            values: BTreeMap::new(),
            switches: BTreeSet::new(),
        };
        let switches = [
            "json",
            "help",
            "if-absent",
            "include-archived",
            "passphrase-stdin",
        ];
        let values = [
            "root",
            "project",
            "name",
            "actor",
            "lock-timeout-ms",
            "file",
            "expected-revision",
            "kind",
            "environment",
            "tag",
            "limit",
            "cursor",
            "format",
            "method",
            "observed-at",
            "evidence-ref",
            "from-commit",
            "output",
            "input",
            "to",
            "id",
            "expected-store-id",
            "expected-project",
        ];
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if arg == "--" {
                parsed.positional.extend_from_slice(&args[index + 1..]);
                break;
            }
            if arg == "-h" {
                parsed.switches.insert("help".into());
                index += 1;
                continue;
            }
            if let Some(key) = arg.strip_prefix("--") {
                if switches.contains(&key) {
                    if !parsed.switches.insert(key.into()) {
                        return Err(invalid("duplicate option"));
                    }
                } else if values.contains(&key) {
                    index += 1;
                    let value = args
                        .get(index)
                        .filter(|value| !value.starts_with("--"))
                        .ok_or_else(|| invalid("option requires a value"))?;
                    let existing = parsed.values.entry(key.into()).or_default();
                    if !existing.is_empty() && !["tag", "id"].contains(&key) {
                        return Err(invalid("duplicate option"));
                    }
                    existing.push(value.clone());
                } else {
                    return Err(invalid("unknown option"));
                }
            } else if arg.starts_with('-') {
                return Err(invalid("unknown option"));
            } else {
                parsed.positional.push(arg.clone());
            }
            index += 1;
        }
        Ok(parsed)
    }
    pub fn flag(&self, key: &str) -> bool {
        self.switches.contains(key)
    }
    pub fn value(&self, key: &str) -> Option<&str> {
        self.values
            .get(key)
            .and_then(|values| values.first())
            .map(String::as_str)
    }
    pub fn required(&self, key: &str) -> Result<&str> {
        self.value(key)
            .ok_or_else(|| invalid("required option is missing"))
    }
    pub fn many(&self, key: &str) -> Vec<String> {
        self.values.get(key).cloned().unwrap_or_default()
    }
    pub fn number(&self, key: &str, default: usize, max: usize) -> Result<usize> {
        let n = self
            .value(key)
            .map(str::parse::<usize>)
            .transpose()
            .map_err(|_| invalid("invalid numeric option"))?
            .unwrap_or(default);
        if n > max {
            return Err(invalid("numeric option exceeds limit"));
        }
        Ok(n)
    }
    pub fn allow(&self, names: &[&str]) -> Result<()> {
        for key in self.values.keys().chain(self.switches.iter()) {
            if !["root", "json", "help"].contains(&key.as_str()) && !names.contains(&key.as_str()) {
                return Err(invalid("option is not valid for this command"));
            }
        }
        Ok(())
    }
    pub fn arity(&self, count: usize) -> Result<()> {
        if self.positional.len() == count {
            Ok(())
        } else {
            Err(invalid("unexpected or missing positional argument"))
        }
    }
}
pub fn invalid(message: &str) -> ContextError {
    ContextError::new("INVALID_INPUT", message)
}
