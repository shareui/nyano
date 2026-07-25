use crate::cxx::bithash;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

pub struct Document {
    pub lines: Vec<Vec<char>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub selection_anchor: Option<Position>,
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    last_edit_kind: EditKind,
    // per-line hash of the file as it was when opened, used to detect real unsaved changes
    original_hashes: Vec<u64>,
    // per-line hash of the current buffer, kept in sync incrementally on every edit
    current_hashes: Vec<u64>,
    tab_width: usize,
}

// snapshot-based undo entry, coalesced by edit kind so continuous typing is one undo step
struct UndoEntry {
    lines: Vec<Vec<char>>,
    cursor_row: usize,
    cursor_col: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum EditKind {
    None,
    Insert,
    Delete,
    Other,
}

// hashes a line's chars as utf-8 bytes, avoiding a string allocation
fn hash_line(line: &[char]) -> u64 {
    let mut buf = [0u8; 4];
    let mut hasher = bithash::Hasher::new(bithash::SEED_DEFAULT);
    for c in line {
        hasher.update(c.encode_utf8(&mut buf).as_bytes());
    }
    hasher.finish()
}

fn hash_all_lines(lines: &[Vec<char>]) -> Vec<u64> {
    lines.iter().map(|line| hash_line(line)).collect()
}

fn expand_tabs(line: &str, tab_width: usize) -> Vec<char> {
    let mut result = Vec::with_capacity(line.len());
    for c in line.chars() {
        if c == '\t' {
            for _ in 0..tab_width {
                result.push(' ');
            }
        } else {
            result.push(c);
        }
    }
    result
}

impl Document {
    pub fn from_text(text: &str, tab_width: usize) -> Self {
        let lines: Vec<Vec<char>> = if text.is_empty() {
            vec![Vec::new()]
        } else {
            text.lines().map(|line| expand_tabs(line, tab_width)).collect()
        };
        let lines = if lines.is_empty() { vec![Vec::new()] } else { lines };
        let original_hashes = hash_all_lines(&lines);
        let current_hashes = original_hashes.clone();
        Document {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit_kind: EditKind::None,
            original_hashes,
            current_hashes,
            tab_width,
        }
    }

    // помойся
    pub fn is_dirty(&self) -> bool {
        self.current_hashes != self.original_hashes
    }

    pub fn mark_saved(&mut self) {
        self.original_hashes.clone_from(&self.current_hashes);
    }

    fn rehash_line(&mut self, row: usize) {
        self.current_hashes[row] = hash_line(&self.lines[row]);
    }

    fn rehash_all(&mut self) {
        self.current_hashes = hash_all_lines(&self.lines);
    }

    pub fn to_text(&self) -> String {
        self.lines
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn snapshot_for(&mut self, kind: EditKind) {
        self.redo_stack.clear();
        if self.last_edit_kind == kind {
            return;
        }
        self.undo_stack.push(UndoEntry {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        self.last_edit_kind = kind;
    }

    pub fn insert_char(&mut self, c: char) {
        self.snapshot_for(EditKind::Insert);
        self.delete_selection_if_any();
        let row = self.cursor_row;
        let col = self.cursor_col;
        self.lines[row].insert(col, c);
        self.cursor_col += 1;
        self.rehash_line(row);
    }

    pub fn insert_tab(&mut self) {
        self.snapshot_for(EditKind::Other);
        for _ in 0..self.tab_width {
            self.insert_char(' ');
        }
    }

    pub fn insert_newline(&mut self) {
        self.snapshot_for(EditKind::Other);
        self.delete_selection_if_any();
        let row = self.cursor_row;
        let col = self.cursor_col;
        let rest = self.lines[row].split_off(col);
        self.rehash_line(row);
        self.current_hashes.insert(row + 1, hash_line(&rest));
        self.lines.insert(row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    pub fn backspace(&mut self) {
        self.snapshot_for(EditKind::Delete);
        if self.delete_selection_if_any() {
            return;
        }
        if self.cursor_col > 0 {
            let row = self.cursor_row;
            self.lines[row].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.rehash_line(row);
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.current_hashes.remove(self.cursor_row);
            self.cursor_row -= 1;
            let prev_len = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].extend(current_line);
            self.cursor_col = prev_len;
            self.rehash_line(self.cursor_row);
        }
    }

    pub fn delete_forward(&mut self) {
        self.snapshot_for(EditKind::Delete);
        if self.delete_selection_if_any() {
            return;
        }
        let row = self.cursor_row;
        if self.cursor_col < self.lines[row].len() {
            self.lines[row].remove(self.cursor_col);
            self.rehash_line(row);
        } else if row + 1 < self.lines.len() {
            let next_line = self.lines.remove(row + 1);
            self.current_hashes.remove(row + 1);
            self.lines[row].extend(next_line);
            self.rehash_line(row);
        }
    }

    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    fn clamp_col(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    pub fn set_cursor_from_screen(&mut self, row: usize, col: usize) {
        let row = row.min(self.lines.len() - 1);
        self.cursor_row = row;
        self.clamp_col_to(col);
    }

    fn clamp_col_to(&mut self, col: usize) {
        let line_len = self.lines[self.cursor_row].len();
        self.cursor_col = col.min(line_len);
    }

    fn cursor_pos(&self) -> Position {
        Position { row: self.cursor_row, col: self.cursor_col }
    }

    pub fn cursor_position(&self) -> Position {
        self.cursor_pos()
    }

    fn extend_selection(&mut self, movement: impl FnOnce(&mut Self)) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos());
        }
        movement(self);
    }

    pub fn select_left(&mut self) {
        self.extend_selection(Self::move_left);
    }

    pub fn select_right(&mut self) {
        self.extend_selection(Self::move_right);
    }

    pub fn select_up(&mut self) {
        self.extend_selection(Self::move_up);
    }

    pub fn select_down(&mut self) {
        self.extend_selection(Self::move_down);
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(Position { row: 0, col: 0 });
        let last_row = self.lines.len() - 1;
        self.cursor_row = last_row;
        self.cursor_col = self.lines[last_row].len();
    }

    pub fn selection_range(&self) -> Option<(Position, Position)> {
        let anchor = self.selection_anchor?;
        let cursor = self.cursor_pos();
        if anchor == cursor {
            return None;
        }
        Some(if anchor < cursor { (anchor, cursor) } else { (cursor, anchor) })
    }

    pub fn selected_text(&self) -> Option<String> {
        let (start, end) = self.selection_range()?;
        Some(self.text_between(start, end))
    }

    fn text_between(&self, start: Position, end: Position) -> String {
        if start.row == end.row {
            return self.lines[start.row][start.col..end.col].iter().collect();
        }
        let mut result = String::new();
        result.extend(&self.lines[start.row][start.col..]);
        for row in start.row + 1..end.row {
            result.push('\n');
            result.extend(&self.lines[row]);
        }
        result.push('\n');
        result.extend(&self.lines[end.row][..end.col]);
        result
    }

    pub fn delete_selection_if_any(&mut self) -> bool {
        let Some((start, end)) = self.selection_range() else {
            self.selection_anchor = None;
            return false;
        };
        self.selection_anchor = None;
        if start.row == end.row {
            self.lines[start.row].drain(start.col..end.col);
        } else {
            let tail = self.lines[end.row].split_off(end.col);
            self.lines.drain(start.row + 1..=end.row);
            self.current_hashes.drain(start.row + 1..=end.row);
            self.lines[start.row].truncate(start.col);
            self.lines[start.row].extend(tail);
        }
        self.rehash_line(start.row);
        self.cursor_row = start.row;
        self.cursor_col = start.col;
        true
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.selected_text()?;
        self.snapshot_for(EditKind::Other);
        self.delete_selection_if_any();
        Some(text)
    }

    pub fn paste(&mut self, text: &str) {
        self.snapshot_for(EditKind::Other);
        self.delete_selection_if_any();
        let mut first = true;
        for chunk in text.split('\n') {
            if !first {
                self.insert_newline_raw();
            }
            first = false;
            for c in chunk.chars() {
                let row = self.cursor_row;
                let col = self.cursor_col;
                self.lines[row].insert(col, c);
                self.cursor_col += 1;
            }
        }
        self.rehash_all();
    }
    
    fn insert_newline_raw(&mut self) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        let rest = self.lines[row].split_off(col);
        self.lines.insert(row + 1, rest);
        self.cursor_row += 1;
        self.cursor_col = 0;
    }

    pub fn undo(&mut self) {
        let Some(entry) = self.undo_stack.pop() else {
            return;
        };
        self.redo_stack.push(UndoEntry {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        self.lines = entry.lines;
        self.cursor_row = entry.cursor_row;
        self.cursor_col = entry.cursor_col;
        self.selection_anchor = None;
        self.last_edit_kind = EditKind::None;
        self.rehash_all();
    }

    pub fn redo(&mut self) {
        let Some(entry) = self.redo_stack.pop() else {
            return;
        };
        self.undo_stack.push(UndoEntry {
            lines: self.lines.clone(),
            cursor_row: self.cursor_row,
            cursor_col: self.cursor_col,
        });
        self.lines = entry.lines;
        self.cursor_row = entry.cursor_row;
        self.cursor_col = entry.cursor_col;
        self.selection_anchor = None;
        self.last_edit_kind = EditKind::None;
        self.rehash_all();
    }
}
