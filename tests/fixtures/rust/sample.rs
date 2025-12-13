// Sample Rust file for testing serialization
use std::collections::HashMap;

/// A simple example struct
pub struct Config {
    pub name: String,
    pub values: HashMap<String, i32>,
}

impl Config {
    /// Creates a new Config instance
    pub fn new(name: &str) -> Self {
        Config {
            name: name.to_string(),
            values: HashMap::new(),
        }
    }

    /// Adds a value to the config
    pub fn add_value(&mut self, key: String, value: i32) {
        self.values.insert(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new("test");
        assert_eq!(config.name, "test");
        assert_eq!(config.values.len(), 0);
    }
}
