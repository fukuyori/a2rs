use std::fs;
use std::path::{Path, PathBuf};

fn is_bare_filename(input: &str) -> bool {
    let path = Path::new(input);
    path.components().count() == 1
}

pub fn candidate_disk_paths(input: &str, disk_dir: &Path, exts: &[&str]) -> Vec<PathBuf> {
    let input_path = Path::new(input);
    let mut candidates = Vec::new();

    if input_path.exists() {
        candidates.push(input_path.to_path_buf());
        return candidates;
    }

    if !is_bare_filename(input) {
        candidates.push(input_path.to_path_buf());
        return candidates;
    }

    let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if input_path.extension().is_some() {
        candidates.push(current_dir.join(input));
        candidates.push(disk_dir.join(input));
    } else {
        for ext in exts {
            let name = format!("{input}.{ext}");
            candidates.push(current_dir.join(&name));
            candidates.push(disk_dir.join(&name));
        }
    }

    candidates
}

pub fn resolve_disk_image(input: &str, disk_dir: &Path, exts: &[&str]) -> Result<PathBuf, Vec<PathBuf>> {
    let candidates = candidate_disk_paths(input, disk_dir, exts);
    if let Some(found) = candidates.iter().find(|p| p.exists()) {
        Ok(found.clone())
    } else {
        Err(candidates)
    }
}

pub fn list_available_disks(disk_dir: &Path, exts: &[&str]) -> Vec<String> {
    let mut disks = Vec::new();
    collect_disk_files(disk_dir, &mut disks, 0, exts);
    disks.sort_by(|a, b| {
        let name_a = Path::new(a)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(a)
            .to_lowercase();
        let name_b = Path::new(b)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(b)
            .to_lowercase();
        name_a.cmp(&name_b)
    });
    disks
}

fn collect_disk_files(dir: &Path, disks: &mut Vec<String>, depth: usize, exts: &[&str]) {
    const MAX_DEPTH: usize = 3;
    if depth > MAX_DEPTH {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(path_str) = path.to_str() {
                    if !path_str.contains("/.") && !path_str.contains("\\.") {
                        collect_disk_files(&path, disks, depth + 1, exts);
                    }
                }
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_ascii_lowercase();
                if exts.iter().any(|allowed| allowed.eq_ignore_ascii_case(&ext_lower)) {
                    if let Some(path_str) = path.to_str() {
                        if !disks.iter().any(|d| d == path_str) {
                            disks.push(path_str.to_string());
                        }
                    }
                }
            }
        }
    }
}
