use crate::column_handler::parse_col_identifier;
use crate::utils::exit_with_error;
use chrono::{Duration, Utc};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

pub fn parse_transform_arg(
    transform_arg: &str,
    header_map: &HashMap<String, usize>,
) -> HashMap<usize, Vec<String>> {
    let parsed: Value =
        serde_json::from_str(transform_arg).expect("Invalid JSON in --transform argument");
    let mut transforms = HashMap::new();
    if let Value::Object(map) = parsed {
        for (key, val) in map {
            let col_index = parse_col_identifier(&key, header_map);
            let ops = match val {
                Value::String(s) => vec![s],
                Value::Array(arr) => arr
                    .into_iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect(),
                _ => exit_with_error("--transform values must be strings or arrays of strings"),
            };
            transforms.insert(col_index, ops);
        }
    }
    transforms
}

pub fn apply_transformers(row: &mut [String], transforms: &HashMap<usize, Vec<String>>) {
    for (&col_idx, trans_list) in transforms {
        if let Some(value) = row.get_mut(col_idx) {
            for transformer in trans_list {
                match transformer.as_str() {
                    "$AGE_TO_DATE" => *value = parse_age_to_date(value),
                    "$TO_LOWER" => *value = value.to_lowercase(),
                    _ => {}
                }
            }
        }
    }
}

pub fn parse_age_to_date(value: &str) -> String {
    let lowercase = value.to_lowercase();
    let re = Regex::new(r"(\d+)[\s]*[a-z]*").unwrap();
    if let Some(cap) = re.captures(&lowercase)
        && let Some(num) = cap.get(1)
    {
        let number: i64 = num.as_str().parse().unwrap_or(0);
        let timestamp = Utc::now() - Duration::days(number);
        return timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
    }
    value.to_string()
}
