use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Table, Wrap};

use crate::app::App;
use crate::model::{AppState, ResultKind};

const BG: Color = Color::Rgb(10, 20, 10);
const FG: Color = Color::Rgb(120, 255, 120);
const DIM: Color = Color::Rgb(70, 160, 70);
const ERROR: Color = Color::Rgb(255, 180, 120);
const PANEL_BG: Color = Color::Rgb(6, 14, 6);
const SELECT_BG: Color = Color::Rgb(35, 110, 35);
const CELL_BG: Color = Color::Rgb(170, 255, 120);

pub fn result_viewport(area: Rect) -> (usize, usize) {
    let app_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
    let result_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(app_chunks[1]);
    let table_inner = block("Rows").inner(result_chunks[1]);

    // Table header consumes one row, and the footer/status line consumes one row.
    let visible_rows = table_inner.height.saturating_sub(2) as usize;
    let visible_cols = visible_column_capacity(usize::MAX, 0, table_inner).max(1);
    (visible_rows, visible_cols)
}

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    frame.render_widget(Block::default().style(Style::default().bg(BG)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    match app.state {
        AppState::Init => render_init(frame, chunks[1], app),
        AppState::QueryList => render_query_list(frame, chunks[1], app),
        AppState::QueryEdit => render_query_edit(frame, chunks[1], app),
        AppState::QueryRunning => render_query_running(frame, chunks[1], app),
        AppState::ResultView => render_result_view(frame, chunks[1], app),
    }
    render_footer(frame, chunks[2], app);

    if app.state == AppState::ResultView && app.result_cell_detail_open {
        render_result_cell_detail(frame, area, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let connection = app
        .current_connection()
        .map(|connection| connection.name.as_str())
        .unwrap_or("NO-LINK");
    let title = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "MRCLI",
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" // ", Style::default().fg(DIM)),
        Span::styled("DATABASE MAINTENANCE TERMINAL", Style::default().fg(FG)),
        Span::styled(" // ", Style::default().fg(DIM)),
        Span::styled(state_name(&app.state), Style::default().fg(DIM)),
        Span::styled(" // ", Style::default().fg(DIM)),
        Span::styled(state_phrase(&app.state), Style::default().fg(FG)),
        Span::styled(" // LINK ", Style::default().fg(DIM)),
        Span::styled(connection, Style::default().fg(FG)),
    ])])
    .style(Style::default().fg(FG).bg(PANEL_BG))
    .block(block("TERMINAL"));
    frame.render_widget(title, area);
}

fn render_init(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let items = if let Some(config) = &app.config {
        config
            .connections
            .iter()
            .enumerate()
            .map(|(index, connection)| {
                let style = if index == app.selected_connection_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(SELECT_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(FG)
                };
                let prefix = if index == app.selected_connection_index {
                    ">> "
                } else {
                    "   "
                };
                ListItem::new(Line::from(format!("{prefix}{}", connection.name))).style(style)
            })
            .collect::<Vec<_>>()
    } else {
        vec![ListItem::new(Line::from("No config loaded")).style(Style::default().fg(ERROR))]
    };

    frame.render_widget(List::new(items).block(block("CONNECTIONS")), chunks[0]);

    let detail_lines = if let Some(connection) = app.current_connection() {
        vec![
            Line::from(format!("kind     : {}", connection.kind)),
            Line::from(format!("host     : {}", connection.host)),
            Line::from(format!("port     : {}", connection.port)),
            Line::from(format!("username : {}", connection.username)),
            Line::from(format!("database : {}", connection.database)),
            Line::from(format!(
                "password : {}",
                mask_password(&connection.password)
            )),
        ]
    } else if let Some(error) = &app.last_error {
        vec![Line::styled(error.clone(), Style::default().fg(ERROR))]
    } else {
        vec![Line::from("No connection available")]
    };

    frame.render_widget(
        Paragraph::new(detail_lines)
            .style(Style::default().fg(FG).bg(BG))
            .block(block("CONNECTION DETAIL"))
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn render_query_list(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let items = if app.drafts.is_empty() {
        vec![ListItem::new(Line::from("No drafts yet"))]
    } else {
        app.drafts
            .iter()
            .enumerate()
            .map(|(index, draft)| {
                let style = if Some(index) == app.selected_draft_index {
                    Style::default()
                        .fg(Color::Black)
                        .bg(SELECT_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(FG)
                };
                let prefix = if Some(index) == app.selected_draft_index {
                    ">> "
                } else {
                    "   "
                };
                ListItem::new(Line::from(format!("{prefix}{}", draft.file_name))).style(style)
            })
            .collect::<Vec<_>>()
    };

    frame.render_widget(List::new(items).block(block("SQL DRAFTS")), chunks[0]);

    let preview = app
        .current_draft()
        .map(|draft| draft.content.clone())
        .filter(|content| !content.is_empty())
        .unwrap_or_else(|| "Draft preview will appear here.".to_string());
    frame.render_widget(
        Paragraph::new(preview)
            .style(Style::default().fg(FG).bg(BG))
            .block(block("SQL PREVIEW"))
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn render_query_edit(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);
    let title = app
        .current_draft()
        .map(|draft| format!("COMMAND INPUT [{}]", draft.file_name))
        .unwrap_or_else(|| "COMMAND INPUT".to_string());

    let draft_name = app
        .current_draft()
        .map(|draft| draft.file_name.as_str())
        .unwrap_or("<unsaved>");
    let connection = app
        .current_connection()
        .map(|connection| connection.name.as_str())
        .unwrap_or("NO-LINK");
    let line_count = app.editor.lines().len();
    let char_count = app
        .editor
        .lines()
        .iter()
        .map(|line| line.chars().count())
        .sum::<usize>();
    let cursor = app.editor.cursor();

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("LINK ", Style::default().fg(DIM)),
            Span::styled(
                connection,
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Span::styled("  |  BUFFER ", Style::default().fg(DIM)),
            Span::styled(draft_name, Style::default().fg(FG)),
            Span::styled("  |  POS ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}:{}", cursor.0 + 1, cursor.1 + 1),
                Style::default().fg(FG),
            ),
            Span::styled("  |  SIZE ", Style::default().fg(DIM)),
            Span::styled(
                format!("{line_count} lines / {char_count} chars"),
                Style::default().fg(FG),
            ),
        ]))
        .style(Style::default().fg(FG).bg(PANEL_BG))
        .block(block("EDITOR STATUS")),
        chunks[0],
    );

    let mut editor = app.editor.clone();
    editor.set_block(block(&title));
    editor.set_style(Style::default().fg(FG).bg(BG));
    editor.set_line_number_style(Style::default().fg(DIM).bg(BG));
    editor.set_cursor_line_style(Style::default().bg(Color::Rgb(14, 36, 14)));
    editor.set_cursor_style(Style::default().fg(Color::Black).bg(CELL_BG));
    editor.set_selection_style(Style::default().fg(Color::Black).bg(SELECT_BG));
    frame.render_widget(&editor, chunks[1]);
}

fn render_query_running(frame: &mut Frame, area: Rect, app: &App) {
    let (connection_name, sql) = app
        .pending_execution
        .as_ref()
        .map(|pending| {
            (
                pending.connection_name.as_str(),
                pending.sql.trim().to_string(),
            )
        })
        .unwrap_or(("No connection", "No SQL queued".to_string()));
    let preview = shorten(&sql, area.width.saturating_mul(3) as usize);

    let popup = centered_rect(70, 45, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "EXECUTING",
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Line::from(""),
            Line::from(format!("connection: {connection_name}")),
            Line::from(""),
            Line::from(preview),
            Line::from(""),
            Line::styled(
                "Running against MySQL in background worker.",
                Style::default().fg(DIM),
            ),
        ])
        .style(Style::default().fg(FG).bg(PANEL_BG))
        .block(block("QUERY RUNNING"))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn render_result_view(frame: &mut Frame, area: Rect, app: &App) {
    let Some(result) = &app.current_result else {
        frame.render_widget(Paragraph::new("No result").block(block("Result")), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    let header = match result.kind {
        ResultKind::Query => format!(
            "RESULT MATRIX | {} ms | {} rows | {} cols",
            result.elapsed_ms,
            result.rows.len(),
            result.columns.len()
        ),
        ResultKind::Command => format!("COMMAND RESULT | {} ms", result.elapsed_ms),
        ResultKind::Error => format!("ERROR | {} ms", result.elapsed_ms),
    };

    let meta = match result.kind {
        ResultKind::Query => {
            let column_count =
                visible_column_capacity(result.columns.len(), app.result_col_offset, chunks[1]);
            let visible_rows = app.result_visible_rows.max(1);
            format!(
                "connection {} | cell r{} c{} | visible rows {}-{} / {} | visible cols {}-{} / {}{}",
                app.current_connection()
                    .map(|connection| connection.name.as_str())
                    .unwrap_or("<none>"),
                app.result_selected_row.saturating_add(1),
                app.result_selected_col.saturating_add(1),
                visible_range_start(result.rows.len(), app.result_row_offset),
                visible_range_end(result.rows.len(), app.result_row_offset, visible_rows),
                result.rows.len(),
                visible_range_start(result.columns.len(), app.result_col_offset),
                visible_range_end(result.columns.len(), app.result_col_offset, column_count),
                result.columns.len(),
                if result.truncated {
                    " | truncated to 200 rows"
                } else {
                    ""
                }
            )
        }
        ResultKind::Command => format!("affected rows: {}", result.affected_rows.unwrap_or(0)),
        ResultKind::Error => "review database error details below".to_string(),
    };

    frame.render_widget(
        Paragraph::new(meta)
            .style(Style::default().fg(DIM).bg(BG))
            .block(block(&header)),
        chunks[0],
    );

    match result.kind {
        ResultKind::Query => render_query_result_table(frame, chunks[1], app),
        ResultKind::Command => {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        "STATEMENT COMPLETED",
                        Style::default().fg(FG).add_modifier(Modifier::BOLD),
                    ),
                    Line::from(""),
                    Line::from(format!(
                        "affected_rows = {}",
                        result.affected_rows.unwrap_or(0)
                    )),
                    Line::from(format!("elapsed_ms    = {}", result.elapsed_ms)),
                    Line::from(""),
                    Line::styled(
                        "Press Esc to return or r to run again.",
                        Style::default().fg(DIM),
                    ),
                ])
                .style(Style::default().fg(FG).bg(BG))
                .block(block("COMMAND DETAIL"))
                .wrap(Wrap { trim: false }),
                chunks[1],
            );
        }
        ResultKind::Error => {
            frame.render_widget(
                Paragraph::new(vec![
                    Line::styled(
                        "QUERY FAILED",
                        Style::default().fg(ERROR).add_modifier(Modifier::BOLD),
                    ),
                    Line::from(""),
                    Line::styled(
                        result
                            .error_message
                            .clone()
                            .unwrap_or_else(|| "Unknown error".to_string()),
                        Style::default().fg(ERROR),
                    ),
                    Line::from(""),
                    Line::styled(
                        "Press e to adjust SQL or r to retry.",
                        Style::default().fg(DIM),
                    ),
                ])
                .style(Style::default().fg(ERROR).bg(BG))
                .block(block("ERROR DETAIL"))
                .wrap(Wrap { trim: false }),
                chunks[1],
            );
        }
    }
}

fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let status = app
        .status_message
        .as_deref()
        .or(app.last_error.as_deref())
        .unwrap_or("Ready");
    let help = match app.state {
        AppState::Init => "Up/Down Move  Enter Open  r Reload Config  q Quit",
        AppState::QueryList => "Up/Down Move  Enter Edit  n New  d Delete  F5 Run  Esc Back",
        AppState::QueryEdit => "Ctrl+S Save  F5 Save+Run  Esc Back",
        AppState::QueryRunning => "q Quit",
        AppState::ResultView => {
            if app.result_cell_detail_open {
                "Enter/Esc Close Detail  q Quit"
            } else {
                "Enter Cell Detail  Esc Back  e Edit  r Rerun  Arrows Move  PgUp/PgDn Page  Home/End Edge"
            }
        }
    };

    let status_style = if app.last_error.is_some() {
        Style::default().fg(ERROR).bg(PANEL_BG)
    } else {
        Style::default().fg(FG).bg(PANEL_BG)
    };

    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled("STATUS ", Style::default().fg(DIM).bg(PANEL_BG)),
            Span::styled(status, status_style),
            Span::styled("  |  KEYS ", Style::default().fg(DIM).bg(PANEL_BG)),
            Span::styled(help, Style::default().fg(FG).bg(PANEL_BG)),
        ])])
        .style(Style::default().fg(FG).bg(PANEL_BG))
        .block(block("SYSTEM")),
        area,
    );
}

fn block(title: &str) -> Block<'_> {
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().fg(FG).bg(BG))
        .border_style(Style::default().fg(DIM))
}

fn render_query_result_table(frame: &mut Frame, area: Rect, app: &App) {
    let Some(result) = &app.current_result else {
        return;
    };

    let outer = block("RESULT ROWS");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);
    let table_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };
    let footer_area = Rect {
        x: inner.x,
        y: inner.y.saturating_add(inner.height.saturating_sub(1)),
        width: inner.width,
        height: 1,
    };

    if result.columns.is_empty() {
        frame.render_widget(Paragraph::new("(no columns)"), table_area);
        return;
    }

    let visible_columns = result
        .columns
        .iter()
        .skip(app.result_col_offset)
        .take(visible_column_capacity(
            result.columns.len(),
            app.result_col_offset,
            inner,
        ))
        .cloned()
        .collect::<Vec<_>>();

    if visible_columns.is_empty() {
        frame.render_widget(
            Paragraph::new("No visible columns at current offset")
                .style(Style::default().fg(ERROR)),
            table_area,
        );
        return;
    }

    let data_widths = visible_columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let source_index = app.result_col_offset + index;
            let max_cell_width = result
                .rows
                .iter()
                .skip(app.result_row_offset)
                .take(app.result_visible_rows.max(1))
                .filter_map(|row| row.get(source_index))
                .map(|value| value.chars().count())
                .max()
                .unwrap_or(0);
            let target = column.chars().count().max(max_cell_width).clamp(8, 32);
            let available_width = table_area
                .width
                .saturating_sub(visible_columns.len() as u16 + 7)
                as usize;
            let fair_share = if visible_columns.is_empty() {
                8
            } else {
                (available_width / visible_columns.len()).max(8)
            };
            let width = target.min(fair_share.max(8)) as u16;
            Constraint::Length(width)
        })
        .collect::<Vec<_>>();
    let mut widths = vec![Constraint::Length(6)];
    widths.extend(data_widths.iter().cloned());

    let header_row = Row::new(
        std::iter::once(
            Cell::from("#").style(Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        )
        .chain(visible_columns.iter().map(|column| {
            Cell::from(column.clone()).style(Style::default().fg(FG).add_modifier(Modifier::BOLD))
        }))
        .collect::<Vec<_>>(),
    )
    .style(
        Style::default()
            .fg(FG)
            .bg(PANEL_BG)
            .add_modifier(Modifier::BOLD),
    );

    let rows = result
        .rows
        .iter()
        .skip(app.result_row_offset)
        .take(app.result_visible_rows.max(1))
        .enumerate()
        .map(|(visible_index, row)| {
            let row_number = app.result_row_offset + visible_index + 1;
            let mut cells = std::iter::once(
                Cell::from(format!("{:>4}", row_number)).style(Style::default().fg(DIM)),
            )
            .chain(
                row.iter()
                    .skip(app.result_col_offset)
                    .take(visible_columns.len())
                    .zip(data_widths.iter())
                    .map(|(value, width)| {
                        Cell::from(truncate_cell(value, width_to_cell_len(width)))
                    }),
            )
            .collect::<Vec<_>>();
            let is_selected = row_number.saturating_sub(1) == app.result_selected_row;
            if is_selected {
                let selected_visible_col = app
                    .result_selected_col
                    .saturating_sub(app.result_col_offset);
                if let Some(cell) = cells.get_mut(selected_visible_col + 1) {
                    *cell = cell.clone().style(
                        Style::default()
                            .fg(Color::Black)
                            .bg(CELL_BG)
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }
            let row_style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(SELECT_BG)
                    .add_modifier(Modifier::BOLD)
            } else if visible_index % 2 == 0 {
                Style::default().fg(FG)
            } else {
                Style::default().fg(FG).bg(Color::Rgb(14, 30, 14))
            };
            Row::new(cells).style(row_style)
        })
        .collect::<Vec<_>>();

    let table = Table::new(rows, widths)
        .header(header_row)
        .column_spacing(1)
        .style(Style::default().fg(FG).bg(BG));
    frame.render_widget(table, table_area);

    let footer = if result.truncated {
        "WARN result truncated to first 200 rows | ENTER cell detail"
    } else {
        "ENTER cell detail"
    };
    frame.render_widget(
        Paragraph::new(footer).style(
            Style::default()
                .fg(if result.truncated { ERROR } else { DIM })
                .bg(BG),
        ),
        footer_area,
    );

    if result.rows.is_empty() {
        let empty_rect = centered_rect(30, 20, table_area);
        frame.render_widget(Clear, empty_rect);
        frame.render_widget(
            Paragraph::new("(no rows)")
                .style(Style::default().fg(DIM))
                .block(block("INFO")),
            empty_rect,
        );
    }
}

fn render_result_cell_detail(frame: &mut Frame, area: Rect, app: &App) {
    let Some((column_name, value)) = app.current_result_cell() else {
        return;
    };

    let popup = centered_rect(72, 60, area);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!("COLUMN: {column_name}"),
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                format!(
                    "ROW: {}  COL: {}",
                    app.result_selected_row.saturating_add(1),
                    app.result_selected_col.saturating_add(1)
                ),
                Style::default().fg(DIM),
            ),
            Line::from(""),
            Line::from(value.to_string()),
            Line::from(""),
            Line::styled("Press Enter or Esc to close.", Style::default().fg(DIM)),
        ])
        .style(Style::default().fg(FG).bg(PANEL_BG))
        .block(block("CELL DETAIL"))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

fn state_name(state: &AppState) -> &'static str {
    match state {
        AppState::Init => "INIT",
        AppState::QueryList => "QUERY LIST",
        AppState::QueryEdit => "QUERY EDIT",
        AppState::QueryRunning => "QUERY RUNNING",
        AppState::ResultView => "RESULT VIEW",
    }
}

fn state_phrase(state: &AppState) -> &'static str {
    match state {
        AppState::Init => "BOOT SEQUENCE",
        AppState::QueryList => "QUERY BUFFER",
        AppState::QueryEdit => "COMMAND COMPOSER",
        AppState::QueryRunning => "EXECUTION CORE ACTIVE",
        AppState::ResultView => "DATA MATRIX",
    }
}

fn mask_password(password: &str) -> String {
    if password.is_empty() {
        return "<empty>".to_string();
    }
    "*".repeat(password.chars().count().min(12))
}

fn shorten(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    value
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn truncate_cell(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    value
        .chars()
        .take(max_len.saturating_sub(3))
        .collect::<String>()
        + "..."
}

fn visible_column_capacity(total_columns: usize, col_offset: usize, area: Rect) -> usize {
    let remaining_columns = total_columns.saturating_sub(col_offset);
    if remaining_columns == 0 {
        return 0;
    }

    let usable_width = area.width.saturating_sub(10) as usize;
    let estimated_columns = (usable_width / 14).max(1);
    remaining_columns.min(estimated_columns)
}

fn width_to_cell_len(width: &Constraint) -> usize {
    match width {
        Constraint::Length(value) => (*value as usize).saturating_sub(1).max(4),
        _ => 16,
    }
}

fn visible_range_start(total: usize, offset: usize) -> usize {
    if total == 0 { 0 } else { offset + 1 }
}

fn visible_range_end(total: usize, offset: usize, page_size: usize) -> usize {
    if total == 0 {
        0
    } else {
        (offset + page_size).min(total)
    }
}
