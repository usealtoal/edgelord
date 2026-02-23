use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn relative_path(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_rs_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| {
        panic!("failed to read dir {}: {e}", dir.display());
    });

    for entry in entries {
        let entry = entry.unwrap_or_else(|e| panic!("failed to read dir entry: {e}"));
        let path = entry.path();

        if path.is_dir() {
            collect_rs_files_recursive(&path, files);
            continue;
        }

        if path.extension().map(|ext| ext == "rs").unwrap_or(false) {
            files.push(path);
        }
    }
}

pub fn collect_rs_files(relative_dir: &str) -> Vec<PathBuf> {
    let base = root().join(relative_dir);
    let mut files = Vec::new();
    collect_rs_files_recursive(&base, &mut files);
    files.sort();
    files
}

pub fn find_lines_containing(
    relative_dir: &str,
    patterns: &[&str],
) -> Vec<(String, usize, String)> {
    let files = collect_rs_files(relative_dir);
    let mut hits = Vec::new();

    for file in files {
        let content = fs::read_to_string(&file).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", file.display());
        });

        for (idx, line) in content.lines().enumerate() {
            if patterns.iter().any(|p| line.contains(p)) {
                hits.push((relative_path(&file), idx + 1, line.to_string()));
            }
        }
    }

    hits
}

pub fn find_lines_containing_except_files(
    relative_dir: &str,
    patterns: &[&str],
    allowed_relative_files: &[&str],
) -> Vec<(String, usize, String)> {
    let allowed: std::collections::HashSet<&str> = allowed_relative_files.iter().copied().collect();
    find_lines_containing(relative_dir, patterns)
        .into_iter()
        .filter(|(path, _, _)| !allowed.contains(path.as_str()))
        .collect()
}

pub fn path_exists(relative_path: &str) -> bool {
    root().join(relative_path).exists()
}

pub fn read_relative(relative_path: &str) -> String {
    fs::read_to_string(root().join(relative_path))
        .unwrap_or_else(|e| panic!("failed to read {relative_path}: {e}"))
}

pub fn find_non_export_lines_in_mod_files(relative_dir: &str) -> Vec<(String, usize, String)> {
    let files = collect_rs_files(relative_dir);
    let mut violations = Vec::new();

    for file in files {
        if file.file_name().and_then(|s| s.to_str()) != Some("mod.rs") {
            continue;
        }

        let content = fs::read_to_string(&file).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", file.display());
        });

        for (idx, raw_line) in content.lines().enumerate() {
            let line = raw_line.trim();

            if line.is_empty()
                || line.starts_with("//")
                || line.starts_with("pub mod ")
                || line.starts_with("mod ")
                || line.starts_with("#[cfg")
            {
                continue;
            }

            violations.push((relative_path(&file), idx + 1, raw_line.to_string()));
        }
    }

    violations
}
