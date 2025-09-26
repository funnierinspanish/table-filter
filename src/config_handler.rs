use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

pub fn config_path() -> PathBuf {
    dirs::config_dir().unwrap().join("tf.config.json")
}

pub fn load_config() -> HashMap<String, Value> {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

pub fn save_config(data: &HashMap<String, Value>) {
    let path = config_path();
    serde_json::to_writer_pretty(&fs::File::create(path).unwrap(), data)
        .expect("Failed to write config file");
}
