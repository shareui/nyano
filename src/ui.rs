use std::io::{self, Write};
use std::path::Path;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use owo_colors::OwoColorize;

use crate::document::{Document, Position};
use crate::editor::{SearchMode, SearchState};
use crate::upaths::truncate_for_display;

pub struct TerminalUi {
    pub width: u16,
    pub height: u16,
    pub scroll_offset: usize,
}

impl TerminalUi {
    pub fn init() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture, Hide)?; // exexe cute!
        let (width, height) = size()?;
        Ok(TerminalUi { width, height, scroll_offset: 0 })
    }

    pub fn shutdown(&self) -> io::Result<()> {
        execute!(io::stdout(), Show, DisableMouseCapture, LeaveAlternateScreen)?;
        disable_raw_mode()
    }

    pub fn refresh_size(&mut self) -> io::Result<()> {
        let (width, height) = size()?;
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn body_height(&self) -> usize {
        self.height.saturating_sub(2) as usize
    }

    pub fn adjust_scroll(&mut self, cursor_row: usize) {
        let body_height = self.body_height().max(1);
        if cursor_row < self.scroll_offset {
            self.scroll_offset = cursor_row;
        } else if cursor_row >= self.scroll_offset + body_height {
            self.scroll_offset = cursor_row - body_height + 1;
        }
    }

    pub fn render(
        &self,
        doc: &Document,
        file_path: &Path,
        read_only: bool,
        banner: Option<&str>,
        version: &str,
        search: &SearchState,
    ) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = io::BufWriter::new(stdout.lock());
        queue!(out, MoveTo(0, 0))?;

        self.render_top_bar(&mut out, version)?;
        self.render_body(&mut out, doc, search)?;
        self.render_status_bar(&mut out, doc, file_path, read_only, banner, search)?;
        if let SearchMode::Input = search.mode {
            self.render_search_box(&mut out, search)?;
        } else {
            self.render_cursor(&mut out, doc)?;
        }

        out.flush()
    }

    fn render_top_bar(&self, out: &mut impl Write, version: &str) -> io::Result<()> {
        let title = format!("Nyano v{version}");
        let centered = pad_or_trim_centered(&title, self.width as usize);
        queue!(out, MoveTo(0, 0))?;
        write!(out, "{}", centered.on_black().white())?;
        Ok(())
    }

    fn render_cursor(&self, out: &mut impl Write, doc: &Document) -> io::Result<()> {
        // guard against cursor_row < scroll_offset (would underflow usize) and
        // clamp to the visible body area
        if doc.cursor_row < self.scroll_offset {
            return Ok(());
        }
        let relative = doc.cursor_row - self.scroll_offset;
        if relative >= self.body_height() {
            return Ok(());
        }
        let screen_row = relative as u16 + 1;
        let screen_col = doc.cursor_col as u16;
        let under_cursor = doc
            .lines
            .get(doc.cursor_row)
            .and_then(|line| line.get(doc.cursor_col))
            .copied()
            .unwrap_or(' ');
        queue!(out, MoveTo(screen_col, screen_row))?;
        write!(out, "{}", under_cursor.reversed())?;
        Ok(())
    }

    fn render_body(&self, out: &mut impl Write, doc: &Document, search: &SearchState) -> io::Result<()> {
        let body_height = self.body_height();
        let selection = doc.selection_range();
        for screen_row in 0..body_height {
            let line_index = self.scroll_offset + screen_row;
            queue!(out, MoveTo(0, (screen_row + 1) as u16))?;
            if line_index < doc.lines.len() {
                let matches = search_cols_on_line(search, line_index);
                self.render_line(out, doc, line_index, selection, &matches)?;
            } else {
                write!(out, "{}", "~".dimmed())?;
            }
        }
        Ok(())
    }

    fn render_line(
        &self,
        out: &mut impl Write,
        doc: &Document,
        line_index: usize,
        selection: Option<(Position, Position)>,
        matches: &[(usize, usize)],
    ) -> io::Result<()> {
        let text: String = doc.lines[line_index].iter().collect();
        let visible = pad_or_trim(&text, self.width as usize);
        let chars: Vec<char> = visible.chars().collect();
        let width = chars.len();

        let selected = selection_cols_on_line(selection, line_index, doc.lines[line_index].len());

        // paint each column with its highlight kind, then run-length write to keep escape codes minimal
        let mut kind = vec![ColKind::Plain; width];
        for &(start, end) in matches {
            for col in start.min(width)..end.min(width) {
                kind[col] = ColKind::SearchMatch;
            }
        }
        if let Some((start, end)) = selected {
            let start = start.min(width);
            let end = end.min(width);
            for col in start..end {
                kind[col] = ColKind::Selected;
            }
        }

        let mut col = 0;
        while col < width {
            let current = kind[col];
            let run_start = col;
            while col < width && kind[col] == current {
                col += 1;
            }
            let run: String = chars[run_start..col].iter().collect();
            match current {
                ColKind::Plain => write!(out, "{run}")?,
                ColKind::SearchMatch => write!(out, "{}", run.on_truecolor(255, 140, 0).black())?,
                ColKind::Selected => write!(out, "{}", run.reversed())?,
            }
        }
        Ok(())
    }

    fn render_status_bar(
        &self,
        out: &mut impl Write,
        doc: &Document,
        file_path: &Path,
        read_only: bool,
        banner: Option<&str>,
        search: &SearchState,
    ) -> io::Result<()> {
        let row = self.height.saturating_sub(1);
        queue!(out, MoveTo(0, row))?;

        if let SearchMode::Active = search.mode {
            let message = search_status_text(search);
            let centered = pad_or_trim_centered(&message, self.width as usize);
            write!(out, "{}", centered.on_truecolor(255, 140, 0).black())?;
            return Ok(());
        }

        if let Some(message) = banner {
            let centered = pad_or_trim_centered(message, self.width as usize);
            write!(out, "{}", centered.on_yellow().black())?;
            return Ok(());
        }

        let position = format!("{}:{}", doc.cursor_row + 1, doc.cursor_col + 1);
        let display_path = truncate_for_display(file_path);
        let mut suffix = String::new();
        if read_only {
            suffix.push_str(" [read-only]");
        }
        if doc.is_dirty() {
            suffix.push_str(" Unsaved changes");
        }

        let width = self.width as usize;
        let mut line: Vec<char> = pad_or_trim_centered(&display_path, width).chars().collect();
        overlay_edge(&mut line, &position, true);
        overlay_edge(&mut line, &suffix, false);

        let text: String = line.into_iter().collect();
        write!(out, "{}", text.on_black().white())?;
        Ok(())
    }

    fn render_search_box(&self, out: &mut impl Write, search: &SearchState) -> io::Result<()> {
        let box_width = (self.width as usize).clamp(20, 50);
        let left = (self.width as usize).saturating_sub(box_width) / 2;
        let top = (self.height as usize).saturating_sub(3) / 2;

        let title = pad_or_trim_centered(" Find ", box_width);
        queue!(out, MoveTo(left as u16, top as u16))?;
        write!(out, "{}", title.on_white().black())?;

        let input_line = format!(" {} ", search.query);
        let mut input_chars: Vec<char> = pad_or_trim(&input_line, box_width).chars().collect();
        let cursor_index = (1 + search.query.chars().count()).min(input_chars.len().saturating_sub(1));
        queue!(out, MoveTo(left as u16, (top + 1) as u16))?;
        for (i, c) in input_chars.drain(..).enumerate() {
            if i == cursor_index {
                write!(out, "{}", c.on_white().black())?;
            } else {
                write!(out, "{}", c.on_black().white())?;
            }
        }

        let hint = pad_or_trim_centered("Enter to search -- esc to cancel", box_width);
        queue!(out, MoveTo(left as u16, (top + 2) as u16))?;
        write!(out, "{}", hint.on_white().black())?;
        Ok(())
    }
}

fn search_status_text(search: &SearchState) -> String {
    if search.matches.is_empty() {
        return "No matches -- ctrl+f to exit search mode".to_string();
    }
    format!(
        "Match {}/{} -- shift+up/down to navigate -- ctrl+f to exit search mode",
        search.current + 1,
        search.matches.len()
    )
}

// writes text at the left or right edge of the line buffer, overwriting those chars in place
fn overlay_edge(line: &mut [char], text: &str, at_start: bool) {
    let chars: Vec<char> = text.chars().collect();
    let width = line.len();
    let len = chars.len().min(width);
    if at_start {
        for i in 0..len {
            line[i] = chars[i];
        }
    } else {
        let offset = width - len;
        for i in 0..len {
            line[offset + i] = chars[i];
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ColKind {
    Plain,
    SearchMatch,
    Selected,
}

fn search_cols_on_line(search: &SearchState, line_index: usize) -> Vec<(usize, usize)> {
    search
        .matches
        .iter()
        .filter(|(start, _)| start.row == line_index)
        .map(|(start, end)| (start.col, end.col))
        .collect()
}

// (start_col, end_col) range
fn selection_cols_on_line(
    selection: Option<(Position, Position)>,
    line_index: usize,
    line_len: usize,
) -> Option<(usize, usize)> {
    let (start, end) = selection?;
    if line_index < start.row || line_index > end.row {
        return None;
    }
    let start_col = if line_index == start.row { start.col } else { 0 };
    let end_col = if line_index == end.row { end.col } else { line_len };
    Some((start_col, end_col))
}

fn pad_or_trim(text: &str, width: usize) -> String {
    let mut chars: Vec<char> = text.chars().collect();
    if chars.len() > width {
        chars.truncate(width);
    }
    let mut result: String = chars.into_iter().collect();
    while result.chars().count() < width {
        result.push(' ');
    }
    result
}

fn pad_or_trim_centered(text: &str, space: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= space {
        let truncated: String = chars.into_iter().take(space).collect();
        return truncated;
    }
    let total_pad = space - chars.len();
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}