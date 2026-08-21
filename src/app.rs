use std::time::Instant;
use std::{sync::mpsc, thread};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_textarea::{CursorMove, Input, TextArea};

use crate::completion::{self, CompletionItem};
use crate::config;
use crate::db;
use crate::drafts;
use crate::model::{
    AppConfig, AppState, DraftEntry, EditorMode, ExecutionHandle, ExecutionResult,
    PendingExecution, ResultKind,
};

pub struct App {
    pub state: AppState,
    pub config: Option<AppConfig>,
    pub selected_connection_index: usize,
    pub drafts: Vec<DraftEntry>,
    pub selected_draft_index: Option<usize>,
    pub editor: TextArea<'static>,
    pub editor_mode: EditorMode,
    pub command_buffer: String,
    pub completion_open: bool,
    pub completion_items: Vec<CompletionItem>,
    pub completion_selected_index: usize,
    pub completion_prefix: String,
    pub current_result: Option<ExecutionResult>,
    pub status_message: Option<String>,
    pub last_error: Option<String>,
    pub result_row_offset: usize,
    pub result_col_offset: usize,
    pub result_selected_row: usize,
    pub result_selected_col: usize,
    pub result_cell_detail_open: bool,
    pub result_visible_rows: usize,
    pub result_visible_cols: usize,
    pub should_quit: bool,
    pub pending_execution: Option<PendingExecution>,
    pub execution_handle: Option<ExecutionHandle>,
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            state: AppState::Init,
            config: None,
            selected_connection_index: 0,
            drafts: Vec::new(),
            selected_draft_index: None,
            editor: Self::new_editor(String::new()),
            editor_mode: EditorMode::Normal,
            command_buffer: String::new(),
            completion_open: false,
            completion_items: Vec::new(),
            completion_selected_index: 0,
            completion_prefix: String::new(),
            current_result: None,
            status_message: None,
            last_error: None,
            result_row_offset: 0,
            result_col_offset: 0,
            result_selected_row: 0,
            result_selected_col: 0,
            result_cell_detail_open: false,
            result_visible_rows: 20,
            result_visible_cols: 1,
            should_quit: false,
            pending_execution: None,
            execution_handle: None,
        };
        app.reload_config();
        app.reload_drafts();
        app
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.state {
            AppState::Init => self.handle_init_key(key),
            AppState::QueryList => self.handle_query_list_key(key),
            AppState::QueryEdit => self.handle_query_edit_key(key),
            AppState::QueryRunning => self.handle_query_running_key(key),
            AppState::ResultView => self.handle_result_view_key(key),
        }
    }

    pub fn on_tick(&mut self) {
        if self.state != AppState::QueryRunning {
            return;
        }

        let Some(handle) = &self.execution_handle else {
            return;
        };

        match handle.receiver.try_recv() {
            Ok(result) => {
                let summary = match result.kind {
                    ResultKind::Query => format!("query returned {} rows", result.rows.len()),
                    ResultKind::Command => format!(
                        "statement affected {} rows",
                        result.affected_rows.unwrap_or(0)
                    ),
                    ResultKind::Error => "query failed".to_string(),
                };
                self.current_result = Some(result);
                self.execution_handle = None;
                self.pending_execution = None;
                self.last_error = None;
                self.result_row_offset = 0;
                self.result_col_offset = 0;
                self.result_selected_row = 0;
                self.result_selected_col = 0;
                self.result_cell_detail_open = false;
                self.status_message = Some(summary);
                self.state = AppState::ResultView;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                let elapsed_ms = self
                    .pending_execution
                    .as_ref()
                    .map(|pending| pending.started_at.elapsed().as_millis())
                    .unwrap_or(0);
                self.current_result = Some(ExecutionResult::error(
                    "query worker disconnected unexpectedly",
                    elapsed_ms,
                ));
                self.execution_handle = None;
                self.pending_execution = None;
                self.status_message = Some("query worker disconnected".to_string());
                self.state = AppState::ResultView;
            }
        }
    }

    pub fn current_connection(&self) -> Option<&crate::model::ConnectionProfile> {
        self.config
            .as_ref()
            .and_then(|config| config.connections.get(self.selected_connection_index))
    }

    pub fn current_draft(&self) -> Option<&DraftEntry> {
        self.selected_draft_index
            .and_then(|index| self.drafts.get(index))
    }

    fn handle_init_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('r') => self.reload_config(),
            KeyCode::Up => self.move_connection_selection(-1),
            KeyCode::Down => self.move_connection_selection(1),
            KeyCode::Enter => {
                self.reload_drafts();
                self.state = AppState::QueryList;
                self.status_message = Some("entered draft list".to_string());
            }
            _ => {}
        }
    }

    fn handle_query_list_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.state = AppState::Init,
            KeyCode::Up => self.move_draft_selection(-1),
            KeyCode::Down => self.move_draft_selection(1),
            KeyCode::Enter => self.open_selected_draft(),
            KeyCode::Char('n') => self.create_and_open_draft(),
            KeyCode::Char('d') => self.delete_selected_draft(),
            KeyCode::F(5) => self.start_execution_from_selected_draft(),
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }
    }

    fn handle_query_edit_key(&mut self, key: KeyEvent) {
        match self.editor_mode {
            EditorMode::Normal => self.handle_query_edit_normal_key(key),
            EditorMode::Insert => self.handle_query_edit_insert_key(key),
            EditorMode::Command => self.handle_query_edit_command_key(key),
        }
    }

    fn handle_query_edit_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::QueryList;
                self.status_message = Some("returned to draft list".to_string());
            }
            KeyCode::Char('i') => {
                self.editor_mode = EditorMode::Insert;
                self.close_completion();
                self.status_message = Some("insert mode".to_string());
            }
            KeyCode::Char('a') => {
                self.editor.move_cursor(CursorMove::Forward);
                self.editor_mode = EditorMode::Insert;
                self.close_completion();
                self.status_message = Some("append mode".to_string());
            }
            KeyCode::Char('o') => {
                self.editor.move_cursor(CursorMove::End);
                self.editor.insert_newline();
                self.editor_mode = EditorMode::Insert;
                self.close_completion();
                self.status_message = Some("opened new line".to_string());
            }
            KeyCode::Char(':') => {
                self.command_buffer.clear();
                self.close_completion();
                self.editor_mode = EditorMode::Command;
                self.status_message = Some("command mode".to_string());
            }
            KeyCode::Char('h') | KeyCode::Left => self.editor.move_cursor(CursorMove::Back),
            KeyCode::Char('j') | KeyCode::Down => self.editor.move_cursor(CursorMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.editor.move_cursor(CursorMove::Up),
            KeyCode::Char('l') | KeyCode::Right => self.editor.move_cursor(CursorMove::Forward),
            KeyCode::Char('0') | KeyCode::Home => self.editor.move_cursor(CursorMove::Head),
            KeyCode::Char('$') | KeyCode::End => self.editor.move_cursor(CursorMove::End),
            KeyCode::Char('G') => self.editor.move_cursor(CursorMove::Bottom),
            KeyCode::Char('x') => {
                self.editor.delete_next_char();
            }
            KeyCode::F(5) => {
                if self.save_current_editor() {
                    self.start_execution_from_editor();
                }
            }
            _ => {}
        }
    }

    fn handle_query_edit_insert_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            if self.completion_open {
                self.close_completion();
            } else {
                self.editor_mode = EditorMode::Normal;
                self.status_message = Some("normal mode".to_string());
            }
            return;
        }

        if self.completion_open {
            match key.code {
                KeyCode::Up => {
                    self.completion_selected_index =
                        self.completion_selected_index.saturating_sub(1);
                    return;
                }
                KeyCode::Down => {
                    self.completion_selected_index = self
                        .completion_selected_index
                        .saturating_add(1)
                        .min(self.completion_items.len().saturating_sub(1));
                    return;
                }
                KeyCode::Tab | KeyCode::Enter => {
                    self.apply_selected_completion();
                    return;
                }
                _ => {}
            }
        }

        if key.code == KeyCode::Char(' ') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.refresh_completion(true);
            return;
        }

        if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.save_current_editor();
            return;
        }

        if key.code == KeyCode::F(5) {
            if self.save_current_editor() {
                self.start_execution_from_editor();
            }
            return;
        }

        self.editor.input(Input::from(key));
        self.refresh_completion(false);
    }

    fn handle_query_edit_command_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.command_buffer.clear();
                self.close_completion();
                self.editor_mode = EditorMode::Normal;
                self.status_message = Some("normal mode".to_string());
            }
            KeyCode::Enter => self.execute_editor_command(),
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(value) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.command_buffer.push(value);
                }
            }
            _ => {}
        }
    }

    fn handle_query_running_key(&mut self, key: KeyEvent) {
        if matches!(key.code, KeyCode::Char('q')) {
            self.should_quit = true;
        }
    }

    fn handle_result_view_key(&mut self, key: KeyEvent) {
        if self.result_cell_detail_open {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.result_cell_detail_open = false;
                }
                KeyCode::Char('q') => self.should_quit = true,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => self.state = AppState::QueryList,
            KeyCode::Enter => {
                if self.current_result_cell().is_some() {
                    self.result_cell_detail_open = true;
                }
            }
            KeyCode::Char('e') => {
                if self.selected_draft_index.is_some() {
                    self.editor_mode = EditorMode::Normal;
                    self.command_buffer.clear();
                    self.state = AppState::QueryEdit;
                }
            }
            KeyCode::Char('r') => self.start_execution_from_editor(),
            KeyCode::Up => {
                self.move_result_selection_up();
            }
            KeyCode::Down => {
                self.move_result_selection_down();
            }
            KeyCode::Left => {
                self.move_result_selection_left();
            }
            KeyCode::Right => {
                self.move_result_selection_right();
            }
            KeyCode::PageUp => {
                self.result_row_offset = self
                    .result_row_offset
                    .saturating_sub(self.result_visible_rows.max(1));
                self.result_selected_row = self
                    .result_selected_row
                    .saturating_sub(self.result_visible_rows.max(1));
            }
            KeyCode::PageDown => {
                let next_offset = self.next_row_offset(self.result_visible_rows.max(1));
                let shift = next_offset.saturating_sub(self.result_row_offset);
                self.result_row_offset = next_offset;
                self.result_selected_row = self
                    .result_selected_row
                    .saturating_add(shift)
                    .min(self.max_selected_row());
            }
            KeyCode::Home => {
                self.result_row_offset = 0;
                self.result_col_offset = 0;
                self.result_selected_row = 0;
                self.result_selected_col = 0;
            }
            KeyCode::End => {
                self.result_row_offset = self.max_row_offset();
                self.result_col_offset = self.max_col_offset();
                self.result_selected_row = self.max_selected_row();
                self.result_selected_col = self.max_selected_col();
            }
            KeyCode::Char('q') => self.should_quit = true,
            _ => {}
        }

        self.normalize_result_selection();
    }

    fn reload_config(&mut self) {
        match config::load_config() {
            Ok(config) => {
                self.selected_connection_index = config::default_connection_index(&config);
                self.config = Some(config);
                self.last_error = None;
                self.status_message = Some("config loaded".to_string());
            }
            Err(error) => {
                self.config = None;
                self.selected_connection_index = 0;
                self.last_error = Some(error.to_string());
                self.status_message = Some("failed to load config".to_string());
            }
        }
    }

    fn reload_drafts(&mut self) {
        match drafts::list_drafts() {
            Ok(drafts_list) => {
                let previous_file = self.current_draft().map(|draft| draft.file_name.clone());
                self.drafts = drafts_list;
                self.selected_draft_index = if self.drafts.is_empty() {
                    None
                } else if let Some(file_name) = previous_file {
                    self.drafts
                        .iter()
                        .position(|draft| draft.file_name == file_name)
                        .or(Some(0))
                } else {
                    Some(0)
                };
                self.last_error = None;
                self.status_message = Some(format!("loaded {} drafts", self.drafts.len()));
            }
            Err(error) => {
                self.drafts.clear();
                self.selected_draft_index = None;
                self.last_error = Some(error.to_string());
                self.status_message = Some("failed to load drafts".to_string());
            }
        }
    }

    fn move_connection_selection(&mut self, delta: isize) {
        let Some(config) = &self.config else {
            return;
        };
        if config.connections.is_empty() {
            return;
        }
        self.selected_connection_index = move_index(
            self.selected_connection_index,
            config.connections.len(),
            delta,
        );
    }

    fn move_draft_selection(&mut self, delta: isize) {
        let Some(index) = self.selected_draft_index else {
            return;
        };
        if self.drafts.is_empty() {
            return;
        }
        self.selected_draft_index = Some(move_index(index, self.drafts.len(), delta));
    }

    fn open_selected_draft(&mut self) {
        let Some((content, file_name)) = self
            .current_draft()
            .map(|draft| (draft.content.clone(), draft.file_name.clone()))
        else {
            self.status_message = Some("no draft selected".to_string());
            return;
        };

        self.editor = Self::new_editor(content);
        self.editor_mode = EditorMode::Normal;
        self.command_buffer.clear();
        self.close_completion();
        self.state = AppState::QueryEdit;
        self.status_message = Some(format!("editing {}", file_name));
    }

    fn create_and_open_draft(&mut self) {
        match drafts::create_new_draft() {
            Ok(created) => {
                let file_name = created.file_name.clone();
                self.reload_drafts();
                self.selected_draft_index = self
                    .drafts
                    .iter()
                    .position(|draft| draft.file_name == file_name)
                    .or(Some(0));
                self.editor = Self::new_editor(String::new());
                self.editor_mode = EditorMode::Normal;
                self.command_buffer.clear();
                self.close_completion();
                self.state = AppState::QueryEdit;
                self.last_error = None;
                self.status_message = Some(format!("created {}", file_name));
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.status_message = Some("failed to create draft".to_string());
            }
        }
    }

    fn delete_selected_draft(&mut self) {
        let Some(draft) = self.current_draft().cloned() else {
            self.status_message = Some("no draft selected".to_string());
            return;
        };

        match drafts::delete_draft(&draft.path) {
            Ok(()) => {
                self.reload_drafts();
                self.last_error = None;
                self.status_message = Some(format!("deleted {}", draft.file_name));
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.status_message = Some("failed to delete draft".to_string());
            }
        }
    }

    fn save_current_editor(&mut self) -> bool {
        let Some(draft) = self.current_draft().cloned() else {
            self.status_message = Some("no draft selected".to_string());
            return false;
        };

        let content = self.editor.lines().join("\n");
        match drafts::save_draft(&draft.path, &content) {
            Ok(()) => {
                self.reload_drafts();
                self.selected_draft_index = self
                    .drafts
                    .iter()
                    .position(|entry| entry.file_name == draft.file_name)
                    .or(Some(0));
                self.last_error = None;
                self.status_message = Some(format!("saved {}", draft.file_name));
                true
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.status_message = Some("failed to save draft".to_string());
                false
            }
        }
    }

    fn start_execution_from_selected_draft(&mut self) {
        let Some(draft) = self.current_draft() else {
            self.status_message = Some("no draft selected".to_string());
            return;
        };

        self.start_execution(draft.content.clone());
    }

    fn start_execution_from_editor(&mut self) {
        self.start_execution(self.editor.lines().join("\n"));
    }

    fn start_execution(&mut self, sql: String) {
        if sql.trim().is_empty() {
            self.current_result = Some(ExecutionResult::error("SQL is empty", 0));
            self.state = AppState::ResultView;
            self.status_message = Some("cannot execute empty SQL".to_string());
            return;
        }

        let Some(connection) = self.current_connection().cloned() else {
            self.current_result = Some(ExecutionResult::error("No connection selected", 0));
            self.state = AppState::ResultView;
            self.status_message = Some("cannot execute without connection".to_string());
            return;
        };

        let (sender, receiver) = mpsc::channel();
        let sql_for_thread = sql.clone();
        let connection_name = connection.name.clone();

        thread::spawn(move || {
            let result = db::execute_sql(&connection, &sql_for_thread);
            let _ = sender.send(result);
        });

        self.pending_execution = Some(PendingExecution {
            started_at: Instant::now(),
            connection_name,
            sql,
        });
        self.execution_handle = Some(ExecutionHandle { receiver });
        self.state = AppState::QueryRunning;
        self.current_result = None;
        self.result_row_offset = 0;
        self.result_col_offset = 0;
        self.result_selected_row = 0;
        self.result_selected_col = 0;
        self.result_cell_detail_open = false;
        self.status_message = Some("executing SQL".to_string());
    }

    fn new_editor(content: String) -> TextArea<'static> {
        let mut editor = if content.is_empty() {
            TextArea::default()
        } else {
            TextArea::from(content.lines().map(ToOwned::to_owned).collect::<Vec<_>>())
        };
        editor.set_style(ratatui::style::Style::default());
        editor.set_cursor_line_style(Default::default());
        editor
    }

    fn execute_editor_command(&mut self) {
        let command = self.command_buffer.trim().to_ascii_lowercase();
        self.command_buffer.clear();

        match command.as_str() {
            "w" => {
                self.save_current_editor();
                self.editor_mode = EditorMode::Normal;
            }
            "q" | "q!" => {
                self.editor_mode = EditorMode::Normal;
                self.state = AppState::QueryList;
                self.status_message = Some("returned to draft list".to_string());
            }
            "wq" | "x" => {
                if self.save_current_editor() {
                    self.editor_mode = EditorMode::Normal;
                    self.state = AppState::QueryList;
                    self.status_message = Some("saved and returned to draft list".to_string());
                } else {
                    self.editor_mode = EditorMode::Normal;
                }
            }
            "run" => {
                self.editor_mode = EditorMode::Normal;
                if self.save_current_editor() {
                    self.start_execution_from_editor();
                }
            }
            "" => {
                self.editor_mode = EditorMode::Normal;
                self.status_message = Some("normal mode".to_string());
            }
            _ => {
                self.editor_mode = EditorMode::Normal;
                self.status_message = Some(format!("unknown command: {command}"));
            }
        }
    }

    fn max_row_offset(&self) -> usize {
        self.current_result
            .as_ref()
            .map(|result| {
                result
                    .rows
                    .len()
                    .saturating_sub(self.result_visible_rows.max(1))
            })
            .unwrap_or(0)
    }

    fn max_col_offset(&self) -> usize {
        self.current_result
            .as_ref()
            .map(|result| {
                result
                    .columns
                    .len()
                    .saturating_sub(self.result_visible_cols.max(1))
            })
            .unwrap_or(0)
    }

    fn next_row_offset(&self, delta: usize) -> usize {
        self.result_row_offset
            .saturating_add(delta)
            .min(self.max_row_offset())
    }

    fn next_col_offset(&self, delta: usize) -> usize {
        self.result_col_offset
            .saturating_add(delta)
            .min(self.max_col_offset())
    }

    fn max_selected_row(&self) -> usize {
        self.current_result
            .as_ref()
            .map(|result| result.rows.len().saturating_sub(1))
            .unwrap_or(0)
    }

    fn max_selected_col(&self) -> usize {
        self.current_result
            .as_ref()
            .map(|result| result.columns.len().saturating_sub(1))
            .unwrap_or(0)
    }

    fn move_result_selection_up(&mut self) {
        if self.result_selected_row == 0 {
            return;
        }

        self.result_selected_row = self.result_selected_row.saturating_sub(1);
        if self.result_selected_row < self.result_row_offset {
            self.result_row_offset = self.result_selected_row;
        }
    }

    fn move_result_selection_down(&mut self) {
        let max_selected = self.max_selected_row();
        if self.result_selected_row >= max_selected {
            return;
        }

        self.result_selected_row += 1;
        let window_end = self.result_row_offset + self.result_visible_rows.max(1);
        if self.result_selected_row >= window_end {
            self.result_row_offset = self.next_row_offset(1);
        }
    }

    fn normalize_result_selection(&mut self) {
        let max_selected = self.max_selected_row();
        self.result_selected_row = self.result_selected_row.min(max_selected);
        self.result_selected_col = self.result_selected_col.min(self.max_selected_col());
        self.result_row_offset = self.result_row_offset.min(self.max_row_offset());
        self.result_col_offset = self.result_col_offset.min(self.max_col_offset());

        if self.result_selected_row < self.result_row_offset {
            self.result_row_offset = self.result_selected_row;
        }

        let window_end = self
            .result_row_offset
            .saturating_add(self.result_visible_rows.max(1));
        if self.result_selected_row >= window_end {
            self.result_row_offset = self
                .result_selected_row
                .saturating_sub(self.result_visible_rows.max(1).saturating_sub(1))
                .min(self.max_row_offset());
        }

        if self.result_selected_col < self.result_col_offset {
            self.result_col_offset = self.result_selected_col;
        }

        let col_window_end = self
            .result_col_offset
            .saturating_add(self.result_visible_cols.max(1));
        if self.result_selected_col >= col_window_end {
            self.result_col_offset = self
                .result_selected_col
                .saturating_sub(self.result_visible_cols.max(1).saturating_sub(1))
                .min(self.max_col_offset());
        }
    }

    fn move_result_selection_left(&mut self) {
        if self.result_selected_col == 0 {
            return;
        }

        self.result_selected_col = self.result_selected_col.saturating_sub(1);
        if self.result_selected_col < self.result_col_offset {
            self.result_col_offset = self.result_selected_col;
        }
    }

    fn move_result_selection_right(&mut self) {
        let max_selected = self.max_selected_col();
        if self.result_selected_col >= max_selected {
            return;
        }

        self.result_selected_col += 1;
        let visible_columns = self.result_visible_cols.max(1);
        let window_end = self.result_col_offset + visible_columns;
        if self.result_selected_col >= window_end {
            self.result_col_offset = self.next_col_offset(1);
        }
    }

    pub fn set_result_viewport(&mut self, visible_rows: usize, visible_cols: usize) {
        self.result_visible_rows = visible_rows.max(1);
        self.result_visible_cols = visible_cols.max(1);
        self.normalize_result_selection();
    }

    pub fn current_result_cell(&self) -> Option<(&str, &str)> {
        let result = self.current_result.as_ref()?;
        let row = result.rows.get(self.result_selected_row)?;
        let column_name = result.columns.get(self.result_selected_col)?;
        let value = row.get(self.result_selected_col)?;
        Some((column_name.as_str(), value.as_str()))
    }

    fn refresh_completion(&mut self, force_open: bool) {
        let prefix = self.current_editor_prefix();
        let items = completion::sql_completions(&prefix);

        if items.is_empty() || (!force_open && prefix.is_empty()) {
            self.close_completion();
            return;
        }

        self.completion_prefix = prefix;
        self.completion_items = items;
        self.completion_selected_index = self
            .completion_selected_index
            .min(self.completion_items.len().saturating_sub(1));
        self.completion_open = true;
    }

    fn close_completion(&mut self) {
        self.completion_open = false;
        self.completion_items.clear();
        self.completion_selected_index = 0;
        self.completion_prefix.clear();
    }

    fn apply_selected_completion(&mut self) {
        let Some(item) = self
            .completion_items
            .get(self.completion_selected_index)
            .cloned()
        else {
            self.close_completion();
            return;
        };

        let prefix_len = self.completion_prefix.chars().count();
        for _ in 0..prefix_len {
            self.editor.move_cursor(CursorMove::Back);
            self.editor.delete_next_char();
        }
        self.editor.insert_str(item.insert_text);
        self.close_completion();
    }

    fn current_editor_prefix(&self) -> String {
        let (row, col) = self.editor.cursor();
        let Some(line) = self.editor.lines().get(row) else {
            return String::new();
        };
        let before_cursor = line.chars().take(col).collect::<String>();
        before_cursor
            .chars()
            .rev()
            .take_while(|value| value.is_ascii_alphanumeric() || *value == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }

    pub fn command_hints(&self) -> Vec<&'static str> {
        completion::command_hints(&self.command_buffer)
    }
}

fn move_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }

    let next = current as isize + delta;
    if next < 0 {
        len - 1
    } else {
        (next as usize) % len
    }
}
