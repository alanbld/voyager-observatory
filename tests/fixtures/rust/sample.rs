// Sample Rust Module

use std::io::{self, Write};
use std::collections::HashMap;

/// Configuration struct
pub struct Config {
    name: String,
    values: HashMap<String, i32>,
}

impl Config {
    pub fn new(name: String) -> Self {
        Config {
            name,
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, key: String, val: i32) {
        self.values.insert(key, val);
    }

    pub fn get(&self, key: &str) -> Option<&i32> {
        self.values.get(key)
    }
}

pub trait Processable {
    fn process(&self) -> Result<(), String>;
    fn validate(&self) -> bool;
}

pub async fn async_handler(data: Vec<u8>) -> Result<(), io::Error> {
    let processed: Vec<u8> = data.iter().map(|x| x * 2).collect();
    io::stdout().write_all(&processed)?;
    Ok(())
}

fn main() {
    let mut config = Config::new("app".to_string());
    config.set("port".to_string(), 8080);
    println!("Config ready");
}
