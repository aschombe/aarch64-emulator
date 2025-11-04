// Copyright (c) 2025 Andrew Schomber
// Licensed under the MIT License. See LICENSE for details.

use crate::assembler::{
    asm_types::Data, parser::data::parse_data_definition, parser::utils::mangle_label,
};

use std::collections::{HashMap, HashSet};

pub fn collect_labels(
    lines: &[String],
    filename: &str,
    global_labels: &HashSet<String>,
) -> HashMap<String, (String, usize)> {
    let mut map = HashMap::new();
    let mut section = "text";
    let mut offset = 0usize;

    for (i, line) in lines.iter().enumerate() {
        let line_content = line.trim();

        if line_content.starts_with(".data") {
            section = "data";
            offset = 0;
        } else if line_content.starts_with(".bss") {
            section = "bss";
            offset = 0;
        } else if line_content.starts_with(".text") {
            section = "text";
            offset = 0;
        }

        if line_content.contains(':') && !line_content.contains(":lo12:") {
            let parts: Vec<&str> = line_content.splitn(2, ':').collect();
            let raw_label = parts[0].trim();
            let is_global = global_labels.contains(raw_label);
            let label = mangle_label(raw_label, filename, is_global);
            map.insert(label, (section.to_string(), offset));

            // If label + data on same line, calculate size
            let rest_of_line = parts.get(1).map(|s| s.trim()).unwrap_or("");
            if section == "data" && !rest_of_line.is_empty() {
                if let Ok(data) = parse_data_definition(rest_of_line, i + 1) {
                    offset += data_size(&data);
                }
            }
        } else if section == "data"
            && !line_content.is_empty()
            && !line_content.starts_with('.')
            && !line_content.ends_with(':')
        {
            if let Ok(data) = parse_data_definition(line_content, i + 1) {
                offset += data_size(&data);
            }
        }
    }
    map
}

fn data_size(data: &Data) -> usize {
    match data {
        Data::Quad(_) => 8,
        Data::Word(_) => 4,
        Data::Byte(_) => 1,
        Data::QuadArr(v) => v.len() * 8,
        Data::WordArr(v) => v.len() * 4,
        Data::IntArr(v) => v.len() * 4,
        Data::ByteArr(v) => v.len(),
        Data::FloatArr(v) => v.len() * 4,
        Data::DoubleArr(v) => v.len() * 8,
        Data::Align(a) => *a,
    }
}
