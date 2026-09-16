use std::collections::HashMap;
use std::path::PathBuf;
use jwalk::WalkDir;

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,        // bytes (recursive for dirs)
    pub item_count: u64,  // number of files inside
    pub is_dir: bool,
}

/// Blocking scan — call from `spawn_blocking`.
/// Uses jwalk parallel traversal; attributes every file's size to its
/// top-level ancestor inside `root`.
pub fn scan_dir(root: PathBuf, show_hidden: bool) -> Vec<Entry> {
    let mut bucket_size: HashMap<PathBuf, u64>  = HashMap::new();
    let mut bucket_count: HashMap<PathBuf, u64> = HashMap::new();

    for result in WalkDir::new(&root).skip_hidden(!show_hidden){
        let Ok(entry) = result else { continue };
        if entry.depth == 0 { continue }

        // Resolve direct child of root that owns this entry
        let rel = match entry.path().strip_prefix(&root) {
            Ok(r)  => r.to_path_buf(),
            Err(_) => continue,
        };
        let top = root.join(rel.components().next().unwrap());

        if entry.file_type().is_file() {
            let sz = entry.metadata().map(|m| m.len()).unwrap_or(0);
            *bucket_size.entry(top.clone()).or_default()  += sz;
            *bucket_count.entry(top).or_default()         += 1;
        } else {
            // Ensure empty dirs still appear in map
            bucket_size.entry(top.clone()).or_default();
            bucket_count.entry(top).or_default();
        }
    }

    let mut entries: Vec<Entry> = std::fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|de| {
            let path     = de.path();
            let name     = de.file_name().to_string_lossy().into_owned();
            let is_dir   = path.is_dir();
            let size     = if is_dir {
                *bucket_size.get(&path).unwrap_or(&0)
            } else {
                de.metadata().map(|m| m.len()).unwrap_or(0)
            };
            let item_count = *bucket_count.get(&path).unwrap_or(&0);
            Entry { path, name, size, item_count, is_dir }
        })
        .collect();

    entries.sort_by(|a, b| b.size.cmp(&a.size));
    entries
}
