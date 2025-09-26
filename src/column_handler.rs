use std::collections::HashMap;

use crate::utils::exit_with_error;

pub fn parse_col_identifier(ident: &str, header_map: &HashMap<String, usize>) -> usize {
    let trimmed = ident.trim_matches('"');
    if let Some(stripped) = trimmed.strip_prefix('$') {
        stripped
            .parse::<usize>()
            .unwrap_or_else(|_| exit_with_error("Invalid column number"))
            - 1
    } else {
        *header_map
            .get(trimmed.to_uppercase().as_str())
            .unwrap_or_else(|| {
                exit_with_error(&format!("Column name '{}' not found in headers", trimmed))
            })
    }
}
