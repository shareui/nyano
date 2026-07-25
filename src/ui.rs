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
    ) -> io::Result<()> {
        let stdout = io::stdout();
        let mut out = io::BufWriter::new(stdout.lock());
        queue!(out, MoveTo(0, 0))?;

        self.render_top_bar(&mut out, version)?;
        self.render_body(&mut out, doc)?;
        self.render_status_bar(&mut out, doc, file_path, read_only, banner)?;
        self.render_cursor(&mut out, doc)?;

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
        let screen_row = (doc.cursor_row - self.scroll_offset) as u16 + 1;
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

    fn render_body(&self, out: &mut impl Write, doc: &Document) -> io::Result<()> {
        let body_height = self.body_height();
        let selection = doc.selection_range();
        for screen_row in 0..body_height {
            let line_index = self.scroll_offset + screen_row;
            queue!(out, MoveTo(0, (screen_row + 1) as u16))?;
            if line_index < doc.lines.len() {
                self.render_line(out, doc, line_index, selection)?;
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
    ) -> io::Result<()> {
        let text: String = doc.lines[line_index].iter().collect();
        let visible = pad_or_trim(&text, self.width as usize);

        let Some(range) = selection_cols_on_line(selection, line_index, doc.lines[line_index].len()) else {
            write!(out, "{visible}")?;
            return Ok(());
        };

        let chars: Vec<char> = visible.chars().collect();
        let width = chars.len();
        let start = range.0.min(width);
        let end = range.1.min(width);
        let before: String = chars[..start].iter().collect();
        let highlighted: String = chars[start..end].iter().collect();
        let after: String = chars[end..].iter().collect();

        write!(out, "{before}")?;
        write!(out, "{}", highlighted.reversed())?;
        write!(out, "{after}")?;
        Ok(())
    }

    fn render_status_bar(
        &self,
        out: &mut impl Write,
        doc: &Document,
        file_path: &Path,
        read_only: bool,
        banner: Option<&str>,
    ) -> io::Result<()> {
        let row = self.height.saturating_sub(1);
        queue!(out, MoveTo(0, row))?;

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
