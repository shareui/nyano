use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use arboard::Clipboard;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};

use crate::backup::create_backup;
use crate::document::{Document, Position};
use crate::ui::TerminalUi;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const BANNER_DURATION: Duration = Duration::from_secs(2);
const NO_MATCH_DURATION: Duration = Duration::from_secs(2);

pub struct Editor {
    doc: Document,
    ui: TerminalUi,
    file_path: PathBuf,
    read_only: bool,
    backup_on_save: bool,
    banner: Option<(String, Instant)>,
    clipboard: Option<Clipboard>,
    search: SearchState,
}

// search feature state machine: closed -> typing query -> browsing matches
pub enum SearchMode {
    Closed,
    Input,
    Active,
}

pub struct SearchState {
    pub mode: SearchMode,
    pub query: String,
    pub matches: Vec<(Position, Position)>,
    pub current: usize,
    pub no_match_shown_at: Option<Instant>,
}

impl SearchState {
    fn new() -> Self {
        SearchState {
            mode: SearchMode::Closed,
            query: String::new(),
            matches: Vec::new(),
            current: 0,
            no_match_shown_at: None,
        }
    }
}

pub enum ExitReason {
    Saved,
    DiscardedChanges,
    PermissionDenied,
}

impl Editor {
    pub fn new(
        doc: Document,
        file_path: PathBuf,
        read_only: bool,
        backup_on_save: bool,
        initial_banner: Option<String>,
    ) -> io::Result<Self> {
        let ui = TerminalUi::init()?;
        let banner = initial_banner.map(|text| (text, Instant::now()));
        let clipboard = Clipboard::new().ok();
        Ok(Editor { doc, ui, file_path, read_only, backup_on_save, banner, clipboard, search: SearchState::new() })
    }

    pub fn run(mut self) -> io::Result<ExitReason> {
        let result = self.main_loop();
        self.ui.shutdown()?;
        result
    }

    fn main_loop(&mut self) -> io::Result<ExitReason> {
        loop {
            self.expire_banner();
            self.expire_no_match();
            self.ui.adjust_scroll(self.doc.cursor_row);
            let banner_text = self.banner.as_ref().map(|(text, _)| text.as_str());
            self.ui.render(&self.doc, &self.file_path, self.read_only, banner_text, VERSION, &self.search)?;

            if !event::poll(Duration::from_millis(200))? {
                continue;
            }

            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Release {
                        continue;
                    }
                    if let Some(reason) = self.handle_key(key.code, key.modifiers)? {
                        return Ok(reason);
                    }
                }
                Event::Mouse(mouse_event) => {
                    let row = self.ui.scroll_offset + mouse_event.row.saturating_sub(1) as usize;
                    let col = mouse_event.column as usize;
                    match mouse_event.kind {
                        MouseEventKind::Down(_) => {
                            self.doc.clear_selection();
                            self.doc.set_cursor_from_screen(row, col);
                        }
                        MouseEventKind::Drag(_) => {
                            if self.doc.selection_anchor.is_none() {
                                self.doc.selection_anchor = Some(self.doc.cursor_position());
                            }
                            self.doc.set_cursor_from_screen(row, col);
                        }
                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    self.ui.refresh_size()?;
                }
                _ => {}
            }
        }
    }

    fn expire_banner(&mut self) {
        if let Some((_, shown_at)) = &self.banner {
            if shown_at.elapsed() >= BANNER_DURATION {
                self.banner = None;
            }
        }
    }

    // auto-closes the search modal after a no-match result has been shown long enough
    fn expire_no_match(&mut self) {
        let Some(shown_at) = self.search.no_match_shown_at else {
            return;
        };
        if shown_at.elapsed() >= NO_MATCH_DURATION {
            self.close_search();
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> io::Result<Option<ExitReason>> {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let shift = modifiers.contains(KeyModifiers::SHIFT);

        if let SearchMode::Input = self.search.mode {
            if ctrl && code == KeyCode::Char('f') {
                self.close_search();
                return Ok(None);
            }
            self.handle_search_input_key(code, shift);
            return Ok(None);
        }

        if let SearchMode::Active = self.search.mode {
            if ctrl && code == KeyCode::Char('f') {
                self.close_search();
                return Ok(None);
            }
            if shift && code == KeyCode::Up {
                self.goto_prev_match();
                return Ok(None);
            }
            if shift && code == KeyCode::Down {
                self.goto_next_match();
                return Ok(None);
            }
        }

        if ctrl && code == KeyCode::Char('f') {
            self.open_search();
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('s') {
            if self.read_only {
                return Ok(None);
            }
            return self.try_save();
        }

        if (ctrl && code == KeyCode::Char('q')) || code == KeyCode::Esc {
            return self.try_quit();
        }

        if ctrl && code == KeyCode::Char('c') {
            self.copy_selection();
            return Ok(None);
        }

        if self.read_only {
            self.handle_navigation_key(code, modifiers);
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('x') {
            self.cut_selection();
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('v') {
            self.paste_clipboard();
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('z') {
            self.doc.undo();
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('y') {
            self.doc.redo();
            return Ok(None);
        }

        if ctrl && code == KeyCode::Char('a') {
            self.doc.select_all();
            return Ok(None);
        }

        match code {
            KeyCode::Char(c) => self.doc.insert_char(c),
            KeyCode::Tab => self.doc.insert_tab(),
            KeyCode::Enter => self.doc.insert_newline(),
            KeyCode::Backspace => self.doc.backspace(),
            KeyCode::Delete => self.doc.delete_forward(),
            _ => self.handle_navigation_key(code, modifiers),
        }
        Ok(None)
    }

    fn handle_navigation_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        let shift = modifiers.contains(KeyModifiers::SHIFT);
        match (code, shift) {
            (KeyCode::Left, true) => self.doc.select_left(),
            (KeyCode::Right, true) => self.doc.select_right(),
            (KeyCode::Up, true) => self.doc.select_up(),
            (KeyCode::Down, true) => self.doc.select_down(),
            (KeyCode::Left, false) => {
                self.doc.clear_selection();
                self.doc.move_left();
            }
            (KeyCode::Right, false) => {
                self.doc.clear_selection();
                self.doc.move_right();
            }
            (KeyCode::Up, false) => {
                self.doc.clear_selection();
                self.doc.move_up();
            }
            (KeyCode::Down, false) => {
                self.doc.clear_selection();
                self.doc.move_down();
            }
            _ => {}
        }
    }

    fn copy_selection(&mut self) {
        let Some(text) = self.doc.selected_text() else {
            return;
        };
        if let Some(clipboard) = self.clipboard.as_mut() {
            let _ = clipboard.set_text(text);
        }
    }

    fn cut_selection(&mut self) {
        let Some(text) = self.doc.cut_selection() else {
            return;
        };
        if let Some(clipboard) = self.clipboard.as_mut() {
            let _ = clipboard.set_text(text);
        }
    }

    fn paste_clipboard(&mut self) {
        let Some(clipboard) = self.clipboard.as_mut() else {
            return;
        };
        if let Ok(text) = clipboard.get_text() {
            self.doc.paste(&text);
        }
    }

    fn try_save(&mut self) -> io::Result<Option<ExitReason>> {
        let text = self.doc.to_text();
        match std::fs::write(&self.file_path, &text) {
            Ok(()) => {
                self.doc.mark_saved();
                if self.backup_on_save {
                    let _ = create_backup(&self.file_path, &text);
                }
                Ok(None)
            }
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                Ok(Some(ExitReason::PermissionDenied))
            }
            Err(_) => Ok(None),
        }
    }

    fn try_quit(&mut self) -> io::Result<Option<ExitReason>> {
        if !self.doc.is_dirty() {
            return Ok(Some(ExitReason::DiscardedChanges));
        }
        match self.prompt_save_on_quit()? {
            QuitChoice::Save => {
                if let Some(reason) = self.try_save()? {
                    return Ok(Some(reason));
                }
                Ok(Some(ExitReason::Saved))
            }
            QuitChoice::Discard => Ok(Some(ExitReason::DiscardedChanges)),
            QuitChoice::Cancel => Ok(None),
        }
    }

    fn prompt_save_on_quit(&mut self) -> io::Result<QuitChoice> {
        loop {
            self.ui.render(
                &self.doc,
                &self.file_path,
                self.read_only,
                Some("Save changes before quitting? (y/n, esc to cancel)"),
                VERSION,
                &self.search,
            )?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(QuitChoice::Save),
                    KeyCode::Char('n') | KeyCode::Char('N') => return Ok(QuitChoice::Discard),
                    KeyCode::Esc => return Ok(QuitChoice::Cancel),
                    _ => {}
                }
            }
        }
    }

    fn open_search(&mut self) {
        self.search.mode = SearchMode::Input;
        self.search.query.clear();
        self.search.matches.clear();
        self.search.current = 0;
        self.search.no_match_shown_at = None;
    }

    fn close_search(&mut self) {
        self.search.mode = SearchMode::Closed;
        self.search.query.clear();
        self.search.matches.clear();
        self.search.current = 0;
        self.search.no_match_shown_at = None;
    }

    fn handle_search_input_key(&mut self, code: KeyCode, _shift: bool) {
        match code {
            KeyCode::Esc => self.close_search(),
            KeyCode::Enter => self.run_search(),
            KeyCode::Backspace => {
                self.search.query.pop();
            }
            KeyCode::Char(c) => self.search.query.push(c),
            _ => {}
        }
    }

    fn run_search(&mut self) {
        self.search.matches = self.doc.find_all(&self.search.query);
        self.search.current = 0;
        self.search.mode = SearchMode::Active;
        if self.search.matches.is_empty() {
            self.search.no_match_shown_at = Some(Instant::now());
            return;
        }
        self.search.no_match_shown_at = None;
        self.jump_to_current_match();
    }

    fn goto_next_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.search.current = (self.search.current + 1) % self.search.matches.len();
        self.jump_to_current_match();
    }

    fn goto_prev_match(&mut self) {
        if self.search.matches.is_empty() {
            return;
        }
        self.search.current = if self.search.current == 0 {
            self.search.matches.len() - 1
        } else {
            self.search.current - 1
        };
        self.jump_to_current_match();
    }

    fn jump_to_current_match(&mut self) {
        let Some((start, _)) = self.search.matches.get(self.search.current) else {
            return;
        };
        self.doc.clear_selection();
        self.doc.set_cursor(*start);
    }
}

enum QuitChoice {
    Save,
    Discard,
    Cancel,
}