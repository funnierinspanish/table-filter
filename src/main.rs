mod column_handler;
mod config_handler;
mod transformers_handler;
mod utils;

use column_handler::parse_col_identifier;
use config_handler::{load_config, save_config};
use transformers_handler::{apply_transformers, parse_transform_arg};
use utils::exit_with_error;

use clap::{CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

#[derive(Parser, Debug)]
#[command(name = "table_filter", about = "Filter and format CLi tabular output")]
struct CLi {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, short, help = "Profile name from the config file to use")]
    profile: Option<String>,

    #[arg(
        short = 'H',
        long = "headers-row",
        help = "1-based row number containing the headers",
        default_value = "1"
    )]
    headers_row: Option<usize>,

    #[arg(
        short = 's',
        long = "skip-lines",
        help = "Number of lines to skip after the header row"
    )]
    skip_lines: Option<usize>,

    #[arg(short = 'r', long = "skip-results", help = "Number of results to skip")]
    skip_results: Option<usize>,

    #[arg(
        short = 'c',
        long = "cols",
        help = "Comma-separated list of column names to display"
    )]
    cols: Option<String>,

    #[arg(
        short = 'f',
        long = "separator",
        help = "Character used to separate columns. The 'Box Drawings Light Vertical' character is used by default",
        default_value = "│"
    )]
    separator: String,

    #[arg(
        short = 'm',
        long = "match",
        help = "JSON object mapping column names to a string or list of strings to match: {\"COLUMN_NAME\": \"value\"}"
    )]
    matcher: Option<String>,

    #[arg(short = 'q', long = "quiet", help = "Only display a column named ID")]
    quiet: bool,

    #[arg(long = "sort-by", help = "Column name to sort by")]
    sort_by: Option<String>,

    #[arg(
        long = "sort-order",
        help = "Sort order (asc or desc)",
        default_value = "asc"
    )]
    sort_order: String,

    #[arg(
        long = "transform",
        help = "JSON object mapping column names to transformation functions. Supported functions: $AGE_TO_DATE, and $TO_LOWER"
    )]
    transform: Option<String>,

    #[arg(long = "no-headers", help="Don't display the headers row", action = clap::ArgAction::SetTrue)]
    no_headers: bool,
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
                _ => exit_with_error("--match values must be strings or lists of strings"),
            };
            matcher.insert(col_index, values);
        }
    }
    matcher
}

fn main() {
    let cli = CLi::parse();

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

    // Show help if no meaningful arguments provided
    if cli.profile.is_none()
        && cli.headers_row.is_none()
        && cli.cols.is_none()
        && profile_data.is_null()
    {
        let mut cmd = CLi::command();
        cmd.print_help().unwrap();
        io::stdout().flush().expect("Failed to flush stdout");
        std::process::exit(1);
    }

    let headers_row = cli
        .headers_row
        .or_else(|| profile_data["headers-row"].as_u64().map(|v| v as usize))
        .expect("Failed to get headers row");

    let skip_lines_count = cli
        .skip_lines
        .or_else(|| profile_data["skip-lines"].as_u64().map(|v| v as usize))
        .unwrap_or(0);

    let skip_results_count = cli
        .skip_results
        .or_else(|| profile_data["skip-results"].as_u64().map(|v| v as usize))
        .unwrap_or(0);

    let cols: Vec<String> = cli
        .cols
        .map(|p| p.split(',').map(|s| s.trim().to_string()).collect())
        .or_else(|| {
            profile_data["cols"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
        })
        .unwrap_or_else(|| {
            eprintln!("Missing --cols");
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
    let show_headers = !(cli.no_headers || profile_data["no-headers"].as_bool().unwrap_or(false));

    let stdin = io::stdin();
    let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();

    let headers: Vec<String> = lines
        .get(headers_row - 1)
        .expect("Invalid headers row")
        .split(&separator)
        .map(|s| s.trim().to_string().to_uppercase())
        .collect();

    let header_map: HashMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), i))
        .collect();

    let row_cols: Vec<usize> = cols
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
        .skip(headers_row + 1 + skip_lines_count) // Skip header row + separator row + additional skip lines
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
            row_cols
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

    // Apply skip_results after sorting
    if skip_results_count > 0 {
        if skip_results_count < rows.len() {
            rows = rows.into_iter().skip(skip_results_count).collect();
        } else {
            rows.clear();
        }
    }

    // Compute max widths
    let mut col_widths: HashMap<usize, usize> = HashMap::new();
    for &col in &row_cols {
        let max = rows
            .iter()
            .map(|r| r.get(col).map_or(0, |v| v.len()))
            .max()
            .unwrap_or(0);
        let header_len = headers.get(col).map_or(0, |h| h.len());
        col_widths.insert(col, std::cmp::max(max, header_len) + 2);
    }

    // Collect all output into a string buffer for better watch compatibility
    let mut output = String::new();

    // Print header
    if show_headers {
        for (i, &col) in row_cols.iter().enumerate() {
            let name = &headers[col];
            output.push_str(&format!("{:width$}", name, width = col_widths[&col]));
            if i < row_cols.len() - 1 {
                output.push_str("  |  ");
            }
        }
        output.push('\n');

        // Separator row
        for (i, &col) in row_cols.iter().enumerate() {
            output.push_str(&format!("{:-<width$}", "", width = col_widths[&col] + 2));
            if i < row_cols.len() - 1 {
                output.push('+');
            }
        }
        output.push('\n');
    }

    // Rows
    for row in rows {
        for (i, &col) in row_cols.iter().enumerate() {
            output.push_str(&format!(
                "{:width$}",
                row.get(col).unwrap_or(&"".to_string()),
                width = col_widths[&col]
            ));
            if i < row_cols.len() - 1 {
                output.push_str("  |  ");
            }
        }
        output.push('\n');
    }

    // Print all output at once
    print!("{}", output);
    io::stdout().flush().expect("Failed to flush stdout");
    std::process::exit(0);
}
