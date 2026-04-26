use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum RomKind {
    Main,
    Disk,
}

#[derive(Debug, Clone)]
pub struct RomSearchResult {
    pub selected: Option<PathBuf>,
    pub candidates: Vec<PathBuf>,
}

fn file_name_lower(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_ascii_lowercase())
}

fn list_rom_candidates(rom_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for dir in rom_search_dirs(rom_dir) {
        files.extend(list_rom_candidates_in_dir(&dir));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.1 == b.1);
    files
}

fn list_rom_candidates_in_dir(rom_dir: &Path) -> Vec<(String, PathBuf)> {
    fs::read_dir(rom_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let ext = path.extension()?.to_str()?.to_ascii_lowercase();
            if ext == "rom" || ext == "bin" {
                file_name_lower(&path).map(|name| (name, path))
            } else {
                None
            }
        })
        .collect()
}

fn rom_search_dirs(rom_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique_dir(&mut dirs, rom_dir.to_path_buf());

    if let Ok(current_dir) = std::env::current_dir() {
        push_unique_dir(&mut dirs, current_dir.join("roms"));
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            push_unique_dir(&mut dirs, exe_dir.join("roms"));
        }
    }

    dirs
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, path: PathBuf) {
    if !dirs.iter().any(|existing| existing == &path) {
        dirs.push(path);
    }
}

fn select_by_priority(candidates: &[(String, PathBuf)], exact: &[&str]) -> Option<PathBuf> {
    for wanted in exact {
        if let Some((_, path)) = candidates.iter().find(|(name, _)| name == wanted) {
            return Some(path.clone());
        }
    }
    candidates.first().map(|(_, path)| path.clone())
}

pub fn find_rom_candidates(rom_dir: &Path, kind: RomKind) -> RomSearchResult {
    let files = list_rom_candidates(rom_dir);

    let filtered: Vec<(String, PathBuf)> = match kind {
        RomKind::Main => files
            .into_iter()
            .filter(|(name, _)| {
                name.starts_with("apple") && (name.ends_with(".rom") || name.ends_with(".bin"))
            })
            .collect(),
        RomKind::Disk => files
            .into_iter()
            .filter(|(name, _)| {
                name.contains("disk") && (name.ends_with(".rom") || name.ends_with(".bin"))
            })
            .collect(),
    };

    let exact = match kind {
        RomKind::Main => &[
            "apple2e.rom",
            "apple2e.bin",
            "apple2ee.rom",
            "apple2ee.bin",
            "apple2plus.rom",
            "apple2plus.bin",
        ][..],
        RomKind::Disk => &[
            "disk2.rom",
            "disk2.bin",
            "disk-ii.rom",
            "disk-ii.bin",
            "diskii.rom",
            "diskii.bin",
        ][..],
    };

    let selected = select_by_priority(&filtered, exact);
    let candidates = filtered.into_iter().map(|(_, path)| path).collect();
    RomSearchResult {
        selected,
        candidates,
    }
}

fn is_bare_filename(input: &str) -> bool {
    let path = Path::new(input);
    path.components().count() == 1
}

pub fn resolve_rom_arg(input: &str, rom_dir: &Path) -> Result<PathBuf, Vec<PathBuf>> {
    let input_path = Path::new(input);
    if input_path.exists() {
        return Ok(input_path.to_path_buf());
    }

    let mut tried = Vec::new();
    if is_bare_filename(input) {
        if let Ok(current_dir) = std::env::current_dir() {
            tried.push(current_dir.join(input));
            tried.push(current_dir.join("roms").join(input));
        }
        tried.push(rom_dir.join(input));
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                tried.push(exe_dir.join("roms").join(input));
            }
        }
    } else {
        tried.push(input_path.to_path_buf());
    }

    if let Some(found) = tried.iter().find(|p| p.exists()) {
        Ok(found.clone())
    } else {
        Err(tried)
    }
}

pub fn log_rom_search_result(kind: RomKind, rom_dir: &Path, result: &RomSearchResult) {
    let kind_name = match kind {
        RomKind::Main => "main",
        RomKind::Disk => "disk",
    };
    if result.candidates.len() > 1 {
        eprintln!(
            "Warning: multiple {} ROM candidates found in {:?}; selecting highest-priority match",
            kind_name, rom_dir
        );
    }
    if result.selected.is_none() {
        eprintln!("Warning: no {} ROM found in {:?}", kind_name, rom_dir);
    }
}
