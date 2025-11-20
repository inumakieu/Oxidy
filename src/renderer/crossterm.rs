use std::io::{self, stdout, Stdout, Write, StdoutLock};

use crossterm::cursor::SetCursorStyle;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{Color, ContentStyle, ResetColor, SetStyle, StyledContent, Stylize};
use crossterm::{cursor::{self, MoveTo}, terminal, QueueableCommand};
use crossterm::{queue, ExecutableCommand};

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

use crate::highlighter::Highlighter;
use crate::plugins::config::Config;
use crate::renderer::{Renderer, Layer};
use crate::buffer::{Buffer, BufferView};
use crate::types::{Token, EditorMode, RenderBuffer, RenderCell, RenderLine, Size, Grid, Rect, ViewId};
use crate::ui::command::Command;
use crate::ui::ui_manager::UiManager;
use crate::editor::Editor;

use crate::log;

pub struct GutterLayer;

impl Layer for GutterLayer {
    fn render(editor: &Editor, view: &BufferView, ui: &UiManager, config: &Config, rect: Rect) -> Grid<RenderCell> {
        let mut grid = Grid::new(
            rect.rows as usize,
            rect.cols as usize,
            RenderCell::blank()
        );

        let active_view = match editor.active_view() {
            Some(v) => v,
            None => {
                for row in 0..rect.rows as usize {
                    grid.cells[row][0] = RenderCell::space(config);
                    grid.cells[row][1] = RenderCell::space(config);
                    grid.cells[row][2] = RenderCell::space(config);
                    grid.cells[row][3] = RenderCell::tilde(config);
                    grid.cells[row][4] = RenderCell::space(config);
                }
                return grid;
            }
        };

        let gutter_width = rect.cols as usize;
        let buffer = editor.buffer(&view.buffer).unwrap();
        let total_lines = buffer.lines.len();

        let scroll = view.scroll.vertical;
        let cursor_line = view.cursor.row;

        let use_relative = config.opt.relative_numbers.unwrap();

        for screen_row in 0..rect.rows as usize {
            let buffer_row = screen_row + scroll;

            if buffer_row >= total_lines {
                for col in 0..gutter_width {
                    grid.cells[screen_row][col] = RenderCell::space(config);
                }
                grid.cells[screen_row][gutter_width - 2] = RenderCell::tilde(config);
                continue;
            }

            let line_number: i32 = if use_relative {
                let dist = (cursor_line as i32 - buffer_row as i32).abs();
                if dist == 0 {
                    (buffer_row + 1) as i32
                } else {
                    dist
                }
            } else {
                (buffer_row + 1) as i32
            };

            let text = format!("{:>width$} ", line_number, width = gutter_width - 1);

            for (i, ch) in text.chars().enumerate() {
                let mut fg = Color::DarkGrey;

                if buffer_row == cursor_line && view.id == active_view.id {
                    fg = config.current_theme().foreground();
                }

                grid.cells[screen_row][i] = RenderCell { 
                    ch: ch, 
                    style: ContentStyle::new()
                        .on(config.current_theme().background())
                        .with(fg),
                    transparent: false
                };
            }
        }

        
        grid
    }
}


pub struct TextLayer;

impl TextLayer {
    fn render_lines(
        grid: &mut Grid<RenderCell>,
        buffer: &Buffer,
        view: &BufferView,
        config: &Config,
        rect: Rect,
    ) {
        let bg = config.current_theme().background();
        let fg = config.current_theme().foreground();

        let first_line = view.scroll.vertical;
        let last_line  = first_line + rect.rows as usize;

        for screen_row in 0..rect.rows as usize {
            let buffer_row = first_line + screen_row;

            if buffer_row >= buffer.lines.len() {
                Self::render_empty_line(&mut grid.cells[screen_row], config);
                continue;
            }

            let text = &buffer.lines[buffer_row];

            let tokens = view.highlighter.highlight(text, buffer_row);

            Self::render_highlighted_line(
                &mut grid.cells[screen_row],
                text,
                &tokens,
                view.scroll.horizontal,
                config
            );
        }
    }

    fn render_empty_line(row: &mut [RenderCell], config: &Config) {
        for cell in row {
            *cell = RenderCell::blank()
        }
    }

    fn render_highlighted_line(
        row: &mut [RenderCell],
        text: &str,
        tokens: &[Token],
        horiz_scroll: usize,
        config: &Config
    ) {
        let mut col = 0;

        for token in tokens {
            let style = ContentStyle::new()
                .on(config.current_theme().background())
                .with(token.style.unwrap_or(config.current_theme().foreground()));

            let mut logical_col = token.offset;

            for ch in token.text.chars() {
                let screen_col = logical_col - horiz_scroll;

                if screen_col >= row.len() { return; }

                row[screen_col] = RenderCell { ch, style, transparent: false };

                logical_col += ch.len_utf8();
            }
        }
    }
}


impl Layer for TextLayer {
    fn render(editor: &Editor, view: &BufferView, ui: &UiManager, config: &Config, rect: Rect) -> Grid<RenderCell> {
        let mut grid = Grid::new(
            rect.rows as usize,
            rect.cols as usize,
            RenderCell::blank()
        );

        let buffer = editor.active_buffer();

        if let Some(buffer) = buffer {
            Self::render_lines(&mut grid, buffer, view, config, rect);
        }

        grid
    }
}

pub struct UiLayer;

impl Layer for UiLayer {
    fn render(editor: &Editor, view: &BufferView, ui: &UiManager, config: &Config, rect: Rect) -> Grid<RenderCell> {
        let mut grid = Grid::new(
            rect.rows as usize,
            rect.cols as usize,
            RenderCell::blank()
        );

        ui.render(&mut grid);

        grid
    }        
}

pub struct Composite;

impl Composite {
    pub fn merge(
        gutter: &Grid<RenderCell>,
        text: &Grid<RenderCell>,
    ) -> Grid<RenderCell> {

        let mut out = gutter.clone();
        
        for row in 0..out.rows() {
            out.cells[row].extend_from_slice(&text.cells[row]);
        }

        out
    }

    pub fn overlay(
        base: &Grid<RenderCell>,
        overlay: &Grid<RenderCell>
    ) -> Grid<RenderCell> {
        let mut out = base.clone();

        for row in 0..overlay.cells.len() {
            for col in 0..overlay.cells[row].len() {
                let cell = overlay.cells[row][col].clone();
                if !cell.transparent {
                    out.cells[row][col] = cell;
                }
            }
        }

        out
    }
}


pub struct CrossTermRenderer {
    pub size: Size,
    pub previous_frame: Grid<RenderCell>,
    pub output: Stdout,
}

impl CrossTermRenderer {
    pub fn new(size: Size) -> Self {
        let mut output = stdout();
        output.execute(terminal::EnterAlternateScreen).expect("Could not enter Alternate Screen.");
        terminal::enable_raw_mode().expect("Could not enable raw mode.");
        output.execute(EnableMouseCapture).expect("Could not enable mouse capture.");

        Self { 
            size: size.clone(),
            previous_frame: Grid::new(
                size.rows as usize,
                size.cols as usize,
                RenderCell::blank()
            ),
            output: output,
        }
    }

    fn draw_frame(&mut self, frame: Grid<RenderCell>, config: &Config) {
        let mut out = self.output.lock();

        queue!(out, MoveTo(0, 0)).unwrap();

        for row in 0..frame.rows() {
            let new_line = &frame.cells[row];

            if let Some(old_line) = self.previous_frame.get(row) {
                if old_line != new_line {
                    self.draw_render_line(&mut out, new_line, config);
                }
            } else {
                self.draw_render_line(&mut out, new_line, config);
            }

            if row + 1 < frame.rows() {
                write!(out, "\r\n").unwrap();
            }
        }

        self.previous_frame = frame;
    }
    
    fn draw_render_line(
        &self,
        output: &mut StdoutLock,
        line: &[RenderCell],
        config: &Config
    ) {

        let mut current_style: Option<ContentStyle> = None;
        let mut printed_cols = 0;

        for cell in line {
            // apply style if needed
            if current_style.as_ref() != Some(&cell.style) {
                queue!(output, SetStyle(cell.style)).ok();
                current_style = Some(cell.style);
            }

            // print the character
            write!(output, "{}", cell.ch).ok();

            // width might be 0,1,2
            let width = cell.ch.len_utf8();
            printed_cols += width;
        }

        // now pad remaining columns
        let total_cols = self.size.cols as usize;

        if printed_cols < total_cols {
            let style = RenderCell::default_style(config);
            queue!(output, SetStyle(style)).ok();

            let missing = total_cols - printed_cols;
            write!(output, "{}", " ".repeat(missing)).ok();
        }

        let _ = queue!(output, ResetColor);
    }
}

impl Renderer for CrossTermRenderer {
    fn begin_frame(&mut self) {
        self.output.queue(terminal::BeginSynchronizedUpdate).expect("Could not begin synchronized update.");
        self.output.queue(cursor::Hide).expect("Could not hide cursor.");
    }

    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config) {
        let gutter_width = 6u16;
        let ui_offset = ui.top_offset();

        let mut horizontal_dir = true;
        let mut prev_x = 0;
        let mut prev_y = 0;

        let mut final_frame = Grid::new(
            self.size.rows as usize,
            self.size.cols as usize,
            RenderCell::space(config)
        );

        for (id, view) in editor.views() {
            let text_width   = view.size.cols - gutter_width;

            let gutter = GutterLayer::render(editor, &view, ui, config, Rect {
                x: prev_x, y: prev_y,
                cols: gutter_width as u16,
                rows: view.size.rows
            });

            let text = TextLayer::render(editor, &view, ui, config, Rect {
                x: prev_x, y: prev_y,
                cols: text_width,
                rows: view.size.rows
            });

            let view_frame = Composite::merge(&gutter, &text);

            final_frame.blit(&view_frame, prev_x as usize, ui_offset + prev_y as usize);

            prev_x += view.size.cols;
        }

        let active_view = editor.active_view();
        if let Some(active_view) = active_view {
            let ui_layer = UiLayer::render(editor, &active_view, ui, config, Rect {
                x: 0, y: 0,
                cols: self.size.cols,
                rows: self.size.rows
            });

            final_frame = Composite::overlay(&final_frame, &ui_layer);
        }

        self.draw_frame(final_frame, config);

        if let Some(active_view) = editor.active_view() {
            let cursor_pos = active_view.cursor.clone();
            let line_length = editor.active_buffer().unwrap().line(cursor_pos.row).unwrap().len();
            
            let col = cursor_pos.col.min(line_length);
            let row = cursor_pos.row  + ui.top_offset()- active_view.scroll.vertical;

            self.output.queue(cursor::MoveTo(gutter_width as u16 + col as u16, row as u16)).expect("Could not move cursor.");
        }
    }

    fn end_frame(&mut self) {
        self.output.queue(cursor::Show).expect("Could not show cursor.");
        self.output.queue(terminal::EndSynchronizedUpdate).expect("Could not end synchronized update.");
        self.output.flush().expect("Could not flush output.");
    }

    fn resize(&mut self, new_size: Size) {
        self.size = new_size;
    }
}

impl Drop for CrossTermRenderer {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode.");
        self.output.execute(terminal::LeaveAlternateScreen).expect("Could not leave alternate screen.");
        self.output.execute(cursor::Show).expect("Could not show cursor.");
        self.output.execute(DisableMouseCapture).expect("Could not disable mouse capture.");
    }
}
