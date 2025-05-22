use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "table_filter", about = "Filter and format CLI tabular output")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, short)]
    profile: Option<String>,

    #[arg(long = "headers-row")]
    headers_row: Option<usize>,

    #[arg(long = "skip")]
    skip: Option<usize>,

    #[arg(long = "print")]
    print: Option<String>,

    #[arg(long = "match")]
    matcher: Option<String>,

    #[arg(long = "separator", default_value = "│")]
    separator: String,

    #[arg(long = "sort-by")]
    sort_by: Option<String>,

    #[arg(long = "sort-order", default_value = "asc")]
    sort_order: String,

    #[arg(long = "transform")]
    transform: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    Set {
        keyval: String,
    },
    Get {
        #[arg(short = 'p')]
        profile: Option<String>,

        #[arg(long = "val")]
        val: bool,

        key: Option<String>,
    },
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap()
        .join("table-formatter.config.json")
}

fn load_config() -> HashMap<String, Value> {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(path).expect("Failed to read config file");
        println!("Config file found");
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_config(data: &HashMap<String, Value>) {
    let path = config_path();
    serde_json::to_writer_pretty(&fs::File::create(path).unwrap(), data)
        .expect("Failed to write config file");
}

fn parse_col_identifier(ident: &str, header_map: &HashMap<String, usize>) -> usize {
    let trimmed = ident.trim_matches('"');
    if let Some(stripped) = trimmed.strip_prefix('$') {
        stripped.parse::<usize>().expect("Invalid column number") - 1
    } else {
        *header_map
            .get(trimmed)
            .unwrap_or_else(|| panic!("Column name '{}' not found in headers", trimmed))
    }
}

fn parse_match_arg(
    match_arg: &str,
    header_map: &HashMap<String, usize>,
) -> HashMap<usize, Vec<String>> {
    let parsed: Value = serde_json::from_str(match_arg).expect("Invalid JSON in --match argument");
    let mut matcher = HashMap::new();
    if let Value::Object(map) = parsed {
        for (key, val) in map {
            let col_index = parse_col_identifier(&key, header_map);
            let values = match val {
                Value::String(s) => vec![s],
                Value::Array(arr) => arr
                    .into_iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect(),
                _ => panic!("--match values must be strings or lists of strings"),
            };
            matcher.insert(col_index, values);
        }
    }
    matcher
}

fn parse_transform_arg(
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
                _ => panic!("--transform values must be strings or arrays of strings"),
            };
            transforms.insert(col_index, ops);
        }
    }
    transforms
}

fn apply_transformers(row: &mut [String], transforms: &HashMap<usize, Vec<String>>) {
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

fn parse_age_to_date(value: &str) -> String {
    let lowercase = value.to_lowercase();
    let re = Regex::new(r"(\d+)[\s]*[a-z]*").unwrap();
    if let Some(cap) = re.captures(&lowercase) {
        if let Some(num) = cap.get(1) {
            let number: i64 = num.as_str().parse().unwrap_or(0);
            let timestamp = Utc::now() - Duration::days(number);
            return timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        }
    }
    value.to_string()
}

fn main() {
    let cli = Cli::parse();

    if let Some(Commands::Config { cmd }) = &cli.command {
        let mut config = load_config();
        match cmd {
            ConfigCmd::Set { keyval } => {
                if let Some((prefix, val)) = keyval.split_once('=') {
                    let (profile, key) = prefix
                        .rsplit_once('.')
                        .expect("Must be in profile.key format");
                    let profile_entry = config.entry(profile.to_string()).or_insert(json!({}));
                    let value = serde_json::from_str(val).unwrap_or_else(|_| json!(val));
                    profile_entry[key] = value;
                    save_config(&config);
                } else {
                    eprintln!("Invalid format. Use profile.key=value");
                }
            }
            ConfigCmd::Get { profile, val, key } => {
                let config = load_config();
                if let Some(p) = profile {
                    if let Some(data) = config.get(p) {
                        println!("{}", serde_json::to_string_pretty(data).unwrap());
                    } else {
                        eprintln!("Profile '{}' not found", p);
                    }
                } else if let Some(k) = key {
                    if let Some((profile, key)) = k.rsplit_once('.') {
                        match config.get(profile).and_then(|p| p.get(key)) {
                            Some(v) => {
                                if *val {
                                    println!("{}", v);
                                } else {
                                    println!("{}={}", key, v);
                                }
                            }
                            None => eprintln!("No such key '{}.{}'", profile, key),
                        }
                    } else {
                        eprintln!("Invalid key format. Use profile.key");
                    }
                } else {
                    eprintln!("Specify either --profile or key");
                }
            }
        }
        return;
    }

    let profile_data = if let Some(profile) = &cli.profile {
        load_config().get(profile).cloned().unwrap_or_default()
    } else {
        Value::Null
    };

    let headers_row = cli
        .headers_row
        .or_else(|| profile_data["headers-row"].as_u64().map(|v| v as usize))
        .unwrap_or_else(|| {
            eprintln!("Missing --headers-row");
            std::process::exit(1)
        });
    let skip = cli
        .skip
        .or_else(|| profile_data["skip"].as_u64().map(|v| v as usize))
        .unwrap_or(0);
    let print: Vec<String> = cli
        .print
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .or_else(|| {
            profile_data["print"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_else(|| {
            eprintln!("Missing --print");
            std::process::exit(1)
        });
    let matcher = cli.matcher.or_else(|| {
        profile_data["match"]
            .as_object()
            .map(|v| Value::Object(v.clone()).to_string())
    });
    let sort_by = cli
        .sort_by
        .or_else(|| profile_data["sort-by"].as_str().map(|v| v.to_string()));
    let sort_order = cli.sort_order;
    let transform = cli.transform.or_else(|| {
        profile_data["transform"]
            .as_object()
            .map(|v| Value::Object(v.clone()).to_string())
    });
    let separator = cli.separator.clone();

    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();

    let headers: Vec<String> = lines
        .get(headers_row - 1)
        .expect("Invalid headers row")
        .split(&separator)
        .map(|s| s.trim().to_string())
        .collect();

    let header_map: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), i))
        .collect();

    let print_cols: Vec<usize> = print
        .iter()
        .map(|col| parse_col_identifier(col, &header_map))
        .collect();
    let matcher = matcher.map(|m| parse_match_arg(&m, &header_map));
    let transforms = transform
        .map(|t| parse_transform_arg(&t, &header_map))
        .unwrap_or_default();
    let sort_col = sort_by.map(|s| parse_col_identifier(&s, &header_map));

    let mut rows: Vec<Vec<String>> = lines
        .iter()
        .skip(skip)
        .map(|line| {
            line.split(&separator)
                .map(|s| s.trim().to_string())
                .collect::<Vec<_>>()
        })
        .filter(|row| {
            matcher.as_ref().is_none_or(|m| {
                m.iter().all(|(col, vals)| {
                    row.get(*col)
                        .is_some_and(|v| vals.iter().any(|f| v.contains(f)))
                })
            })
        })
        .map(|mut row| {
            apply_transformers(&mut row, &transforms);
            row
        })
        // If the row has no values in the columns we want to print, skip it
        .filter(|row| {
            print_cols
                .iter()
                .any(|&col| row.get(col).is_some_and(|v| !v.is_empty()))
        })
        .collect();

    if let Some(col) = sort_col {
        let empty_string = "".to_string();
        rows.sort_by(|a, b| {
            let a_val = a.get(col).unwrap_or(&empty_string);
            let b_val = b.get(col).unwrap_or(&empty_string);
            if sort_order == "desc" {
                b_val.cmp(a_val)
            } else {
                a_val.cmp(b_val)
            }
        });
    }

    // Compute max widths
    let mut col_widths: HashMap<usize, usize> = HashMap::new();
    for &col in &print_cols {
        let max = rows
            .iter()
            .map(|r| r.get(col).map_or(0, |v| v.len()))
            .max()
            .unwrap_or(0);
        let header_len = headers.get(col).map_or(0, |h| h.len());
        col_widths.insert(col, std::cmp::max(max, header_len) + 2);
    }

    // Print header
    for (i, &col) in print_cols.iter().enumerate() {
        let name = &headers[col];
        print!("{:width$}", name, width = col_widths[&col]);
        if i < print_cols.len() - 1 {
            print!("  |  ");
        }
    }
    println!();

    // Separator row
    for (i, &col) in print_cols.iter().enumerate() {
        print!("{:-<width$}", "", width = col_widths[&col] + 2);
        if i < print_cols.len() - 1 {
            print!("+");
        }
    }
    println!();

    // Rows
    for row in rows {
        for (i, &col) in print_cols.iter().enumerate() {
            print!(
                "{:width$}",
                row.get(col).unwrap_or(&"".to_string()),
                width = col_widths[&col]
            );
            if i < print_cols.len() - 1 {
                print!("  |  ");
            }
        }
        println!();
    }
}
