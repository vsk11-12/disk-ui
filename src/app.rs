use std::path::PathBuf;
use tokio::sync::mpsc;
use crate::scanner::{scan_dir, Entry};
use crate::theme::Theme;

pub enum ScanState {
    Scanning,
    Done,
}

pub struct App {
    pub current_path:     PathBuf,
    pub entries:          Vec<Entry>,
    /// Indices into `entries` that pass the current search filter.
    /// When not searching this is always `0..entries.len()`.
    pub filtered_indices: Vec<usize>,
    /// Cursor — an index into `filtered_indices`, not into `entries`.
    pub selected:         usize,
    pub scan_state:       ScanState,
    pub error_msg:        Option<String>,
    pub show_hidden:      bool,
    pub quit_confirm:     bool,
    pub show_help:        bool,
    /// Transient status message shown in the controls bar.
    pub status:           String,
    pub is_searching:     bool,
    pub search_query:     String,
    pub theme:            Theme,
    nav_history:          Vec<(PathBuf, usize)>,
    scan_tx:              Option<mpsc::Sender<()>>,
    result_rx:            Option<mpsc::Receiver<Result<Vec<Entry>, String>>>,
}

impl App {
    pub fn new(path: PathBuf) -> Self {
        Self {
            current_path:     path,
            entries:          Vec::new(),
            filtered_indices: Vec::new(),
            selected:         0,
            scan_state:       ScanState::Scanning,
            error_msg:        None,
            show_hidden:      false,
            quit_confirm:     false,
            show_help:        false,
            status:           String::new(),
            is_searching:     false,
            search_query:     String::new(),
            theme:            Theme::default(),
            nav_history:      Vec::new(),
            scan_tx:          None,
            result_rx:        None,
        }
    }

    // ── Theme ─────────────────────────────────────────────────────────────────

    pub fn reload_theme(&mut self) {
        self.theme  = Theme::default();
        self.status = "Theme reloaded".to_string();
    }

    // ── Search / filter ───────────────────────────────────────────────────────

    pub fn update_search_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.entries.len()).collect();
        } else {
            let q = self.search_query.to_lowercase();
            self.filtered_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.name.to_lowercase().contains(&q))
                .map(|(i, _)| i)
                .collect();
        }
        self.clamp_selection();
    }

    pub fn clamp_selection(&mut self) {
        if self.filtered_indices.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len() - 1;
        }
    }

    /// The entry the cursor currently points at (respects filter).
    pub fn current_entry(&self) -> Option<&Entry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| self.entries.get(i))
    }

    // ── Visibility ────────────────────────────────────────────────────────────

    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.status = format!(
            "Hidden files: {}",
            if self.show_hidden { "ON" } else { "OFF" }
        );
        self.start_scan();
    }

    // ── Scanning ──────────────────────────────────────────────────────────────

    pub fn start_scan(&mut self) {
        self.scan_state   = ScanState::Scanning;
        self.entries.clear();
        self.filtered_indices.clear();
        self.error_msg    = None;
        self.is_searching = false;
        self.search_query.clear();

        let (result_tx, result_rx)      = mpsc::channel(1);
        let (_cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
        let root        = self.current_path.clone();
        let show_hidden = self.show_hidden;

        tokio::spawn(async move {
            let root2  = root.clone();
            let handle = tokio::task::spawn_blocking(move || scan_dir(root2, show_hidden));
            tokio::select! {
                res = handle => {
                    let payload = res.map_err(|e| e.to_string());
                    let _ = result_tx.send(payload).await;
                }
                _ = cancel_rx.recv() => {}
            }
        });

        self.result_rx = Some(result_rx);
        self.scan_tx   = Some(_cancel_tx);
    }

    pub fn poll_scan(&mut self) {
        let Some(rx) = self.result_rx.as_mut() else { return };
        if let Ok(result) = rx.try_recv() {
            match result {
                Ok(entries) => {
                    self.entries = entries;
                    self.status  = format!("{} items found", self.entries.len());
                }
                Err(e) => {
                    self.error_msg = Some(e.clone());
                    self.status    = format!("Scan error: {e}");
                }
            }
            self.scan_state = ScanState::Done;
            self.result_rx  = None;
            // Rebuild filter (no-op when not searching) and clamp cursor.
            self.update_search_filter();
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty()
            && self.selected + 1 < self.filtered_indices.len()
        {
            self.selected += 1;
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn enter(&mut self) {
        let Some(entry) = self.current_entry() else { return };
        if !entry.is_dir { return }

        let new_path = entry.path.clone();
        // Save current filtered-cursor position so we can restore it on go_up.
        self.nav_history.push((self.current_path.clone(), self.selected));
        self.current_path = new_path;
        self.selected     = 0;
        self.start_scan();
    }

    pub fn nav_depth(&self) -> usize {
        self.nav_history.len()
    }

    pub fn go_up(&mut self) {
        if let Some((prev_path, prev_sel)) = self.nav_history.pop() {
            self.current_path = prev_path;
            // Restore cursor — start_scan doesn't reset `selected`, and
            // clamp_selection after the scan will bound-check it.
            self.selected = prev_sel;
            self.start_scan();
        } else if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_path_buf();
            self.selected     = 0;
            self.start_scan();
        }
    }

    // ── Aggregates ────────────────────────────────────────────────────────────

    pub fn total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }
}
