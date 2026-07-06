//! Function and global extraction from C2Rust translated TeX sources.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{PatchError, RawFunction};
use crate::transform::patch_c_types;

pub(crate) fn extract_functions(path: &Path) -> Result<Vec<RawFunction>, PatchError> {
    let input = fs::read_to_string(path)?;
    let lines = input.lines().collect::<Vec<_>>();
    let mut functions = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if lines[index] != "#[no_mangle]" {
            index += 1;
            continue;
        }

        let start = index;
        let signature_index = index + 1;
        let signature =
            lines
                .get(signature_index)
                .ok_or_else(|| PatchError::MalformedFunction {
                    path: path.to_path_buf(),
                    line: start + 1,
                })?;

        let Some(name) = function_name(signature) else {
            index += 1;
            continue;
        };

        let mut depth = 0usize;
        let mut saw_body = false;
        let mut end = None;
        for (offset, line) in lines[start..].iter().enumerate() {
            for byte in line.bytes() {
                match byte {
                    b'{' => {
                        depth += 1;
                        saw_body = true;
                    }
                    b'}' if depth > 0 => {
                        depth -= 1;
                        if saw_body && depth == 0 {
                            end = Some(start + offset);
                        }
                    }
                    _ => {}
                }
            }
            if end.is_some() {
                break;
            }
        }

        let end = end.ok_or_else(|| PatchError::MalformedFunction {
            path: path.to_path_buf(),
            line: start + 1,
        })?;
        let mut source = lines[start..=end].join("\n");
        source.push('\n');
        functions.push(RawFunction { name, source });
        index = end + 1;
    }

    Ok(functions)
}


pub(crate) fn extract_functions_from_paths(paths: &[PathBuf]) -> Result<Vec<RawFunction>, PatchError> {
    let mut functions = Vec::new();
    for path in paths {
        functions.extend(extract_functions(path)?);
    }
    Ok(functions)
}


pub(crate) fn function_name(signature: &str) -> Option<String> {
    let rest = signature.strip_prefix("pub unsafe extern \"C\" fn ")?;
    let end = rest
        .find(|character: char| character == '(' || character.is_whitespace())
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}


pub(crate) fn extract_globals(paths: &[PathBuf]) -> Result<BTreeMap<String, String>, PatchError> {
    let mut globals = BTreeMap::new();
    for path in paths {
        let input = fs::read_to_string(path)?;
        for line in input.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("static mut ") else {
                continue;
            };
            let Some((name, ty)) = rest.split_once(':') else {
                continue;
            };
            let ty = ty.trim().trim_end_matches(';');
            globals.insert(name.to_string(), patch_c_types(ty.to_string()));
        }
    }
    Ok(globals)
}
