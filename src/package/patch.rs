use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

const SKIP_ENV: &str = "HOUDINI_PACKAGE_PATH";

pub fn patch_dir(install_dir: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in WalkDir::new(install_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        match try_patch_file(entry.path(), install_dir) {
            Ok(true) => {
                log::info!("Patched {}", entry.path().display());
                count += 1;
            }
            Ok(false) => {}
            Err(e) => log::debug!("Skipped {}: {e}", entry.path().display()),
        }
    }
    Ok(count)
}

fn try_patch_file(file: &Path, install_dir: &Path) -> Result<bool> {
    let text =
        fs::read_to_string(file).with_context(|| format!("Failed to read {}", file.display()))?;
    let mut json: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(false),
    };
    let obj = match json.as_object_mut() {
        Some(o) => o,
        None => return Ok(false),
    };

    let key = if obj.contains_key("hpath") {
        "hpath"
    } else if obj.contains_key("path") {
        "path"
    } else {
        return Ok(false);
    };

    let raw = match extract_first_string(obj.get(key).unwrap()) {
        Some(s) => s,
        None => return Ok(false),
    };

    let install_str = install_dir.to_string_lossy().into_owned();
    let patched = if let Some(env_var) = raw.strip_prefix('$') {
        if env_var == SKIP_ENV {
            return Ok(false);
        }
        patch_env_entry(obj, env_var, &install_str)
    } else {
        patch_path_field(obj, key, &install_str);
        true
    };

    if patched {
        let new_text = serde_json::to_string_pretty(&json)
            .with_context(|| format!("Failed to serialize {}", file.display()))?;
        fs::write(file, format!("{new_text}\n"))
            .with_context(|| format!("Failed to write {}", file.display()))?;
    }
    Ok(patched)
}

fn extract_first_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Array(arr) => arr.first().and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }),
        _ => None,
    }
}

fn patch_env_entry(obj: &mut Map<String, Value>, var: &str, install_path: &str) -> bool {
    let env = match obj.get_mut("env").and_then(|v| v.as_array_mut()) {
        Some(a) => a,
        None => return false,
    };
    let skip_marker = format!("${SKIP_ENV}");
    for entry in env.iter_mut() {
        let entry_obj = match entry.as_object_mut() {
            Some(o) => o,
            None => continue,
        };
        if !entry_obj.contains_key(var) {
            continue;
        }
        let value = entry_obj.get_mut(var).unwrap();
        let current = match value {
            Value::String(s) => Some(s.as_str()),
            Value::Object(m) => m.get("value").and_then(|v| v.as_str()),
            _ => None,
        };
        if current == Some(skip_marker.as_str()) {
            return false;
        }
        match value {
            Value::Object(m) => {
                m.insert("value".to_string(), Value::String(install_path.to_string()));
            }
            _ => {
                *value = Value::String(install_path.to_string());
            }
        }
        return true;
    }
    false
}

fn patch_path_field(obj: &mut Map<String, Value>, key: &str, install_path: &str) {
    let entry = obj.get_mut(key).unwrap();
    match entry {
        Value::Array(arr) => {
            if let Some(first) = arr.get_mut(0) {
                *first = Value::String(install_path.to_string());
            } else {
                arr.push(Value::String(install_path.to_string()));
            }
        }
        _ => {
            *entry = Value::String(install_path.to_string());
        }
    }
}
