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
use crate::types::{Token, EditorMode, RenderBuffer, RenderCell, RenderLine, Size, Grid};
use crate::ui::command::Command;
use crate::ui::ui_manager::UiManager;
use crate::editor::Editor;

pub struct GutterLayer;

impl Layer for GutterLayer {
    fn render(editor: &Editor, ui: &UiManager, config: &Config, size: Size) -> Grid<RenderCell> {
        let mut grid = Grid::new(
            size.rows as usize,
            size.cols as usize,
            RenderCell::blank()
        );

        let view = match editor.active_view() {
            Some(v) => v,
            None => {
                // Empty buffer case with '~'
                for row in 0..size.rows as usize {
                    grid.cells[row][0] = RenderCell::space(config);
                    grid.cells[row][1] = RenderCell::space(config);
                    grid.cells[row][2] = RenderCell::space(config);
                    grid.cells[row][3] = RenderCell::tilde(config);
                    grid.cells[row][4] = RenderCell::space(config);
                }
                return grid;
            }
        };

        let gutter_width = size.cols as usize;
        let buffer = editor.buffer(&view.buffer).unwrap();
        let total_lines = buffer.lines.len();

        let scroll = view.scroll.vertical;
        let cursor_line = view.cursor.row + scroll; // absolute line index

        let use_relative = config.opt.relative_numbers.unwrap();

        for screen_row in 0..size.rows as usize {
            let buffer_row = screen_row + scroll;

            if buffer_row >= total_lines {
                // draw "~" beyond end of buffer
                for col in 0..gutter_width {
                    grid.cells[screen_row][col] = RenderCell::space(config);
                }
                grid.cells[screen_row][gutter_width - 2] = RenderCell::tilde(config);
                continue;
            }

            // ----- COMPUTE LINE NUMBER -----
            let line_number: i32 = if use_relative {
                let dist = (cursor_line as i32 - buffer_row as i32).abs();
                if dist == 0 {
                    (buffer_row + 1) as i32       // real number for current line
                } else {
                    dist                         // relative for others
                }
            } else {
                (buffer_row + 1) as i32
            };

            let text = format!("{:>width$} ", line_number, width = gutter_width - 1);

            // ----- WRITE INTO GRID -----
            for (i, ch) in text.chars().enumerate() {
                grid.cells[screen_row][i] = RenderCell { ch: ch.into(), style: RenderCell::default_style(config) };
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
        size: Size,
    ) {
        let bg = config.current_theme().background();
        let fg = config.current_theme().foreground();

        let first_line = view.scroll.vertical;
        let last_line  = first_line + size.rows as usize;

        for screen_row in 0..size.rows as usize {
            let buffer_row = first_line + screen_row;

            // If user scrolled past EOF → blank row
            if buffer_row >= buffer.lines.len() {
                Self::render_empty_line(&mut grid.cells[screen_row], bg);
                continue;
            }

            let text = &buffer.lines[buffer_row];

            // highlight tokens for that line
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

    fn render_empty_line(row: &mut [RenderCell], bg: Color) {
        for cell in row {
            *cell = RenderCell::blank();
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
            let style_fg = token.style.unwrap_or(config.current_theme().foreground());
            let style = ContentStyle::new().on(config.current_theme().background()).with(style_fg);

            let mut logical_col = token.offset;

            for g in token.text.graphemes(true) {
                let width = unicode_width::UnicodeWidthStr::width(g);

                if logical_col + width <= horiz_scroll {
                    logical_col += width;
                    continue; // skip scrolled-off characters
                }

                let screen_col = logical_col - horiz_scroll;

                if screen_col >= row.len() { return; }

                // draw main char
                row[screen_col] = RenderCell::from_grapheme(g, style);

                // fill extra width for wide chars
                for i in 1..width {
                    if screen_col + i < row.len() {
                        row[screen_col + i] = RenderCell::space(config);
                    }
                }

                logical_col += width;
            }
        }
    }
}


impl Layer for TextLayer {
    fn render(editor: &Editor, ui: &UiManager, config: &Config, size: Size) -> Grid<RenderCell> {
        let mut grid = Grid::new(
            size.rows as usize,
            size.cols as usize,
            RenderCell::blank()
        );

        let view = match editor.active_view() {
            Some(v) => v,
            None => return grid, // nothing open → blank text area
        };

        let buffer = editor.active_buffer();

        if let Some(buffer) = buffer {
            Self::render_lines(&mut grid, buffer, view, config, size);
        }

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
        
        /*
        // apply cursor on top of text
        apply_layer_modifications(&mut out, cursor);

        // apply inline hints and ghost text
        apply_layer_modifications(&mut out, inline);

        // overlay replaces entire cells
        apply_overlays(&mut out, overlay);
        */
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

        for cell in line {
            if current_style.as_ref() != Some(&cell.style) {
                let _ = queue!(output, SetStyle(cell.style));
                current_style = Some(cell.style);
            }

            let _ = write!(output, "{}", cell.ch);
        }

        let missing = self.size.cols as usize - line.len();
        if missing > 0 {
            let style = RenderCell::default_style(config);
            let _ = queue!(output, SetStyle(style));
            let _ = write!(output, "{}", " ".repeat(missing));
        }

        let _ = queue!(output, ResetColor);
    }

    /*
    fn textfield(&mut self, buffer: &Buffer, highlighter: &mut Highlighter, config: &Config) -> io::Result<()> {
        // TODO: Make colors be based on current theme
        let bg = Color::Rgb { r: 22, g: 22, b: 23 };
        let fg = Color::Rgb { r: 201, g: 199, b: 205 };
        let line_color = Color::Rgb { r: 68, g: 68, b: 72 };

        for row in 0..(self.size.rows - 1) {
            let line = buffer.get_at(row as usize);
            let mut current_render_line = RenderLine { 
                cells: vec![
                    RenderCell { ch: " ".to_string(), style: ContentStyle::new().reset() };
                    self.size.cols as usize
                ]
            };
            if line.is_none() {
                let empty = "    ∼ ".to_string().on(bg.clone()).with(line_color.clone());
                for (index, char) in empty.content().chars().enumerate() {
                    current_render_line.cells[index] = RenderCell { ch: char.to_string(), style: empty.style().clone() };
                }
                self.render_buffer.current[row as usize] = current_render_line;
                continue;
            }

            let line_number = {
                let current_line = buffer.cursor.row as i16 + 1;
                let line_number: StyledContent<String>;
                if config.opt.relative_numbers.unwrap() {
                    let signed_row = row as i16 + 1;
                    let signed_scroll_offset = buffer.scroll_offset.vertical as i16;
                    let relative_distance = (current_line - (signed_row + signed_scroll_offset)).abs();
                    if current_line == signed_row + signed_scroll_offset { 
                        line_number = format!("{:5} ", current_line).on(bg.clone()).with(fg.clone());
                    } else {
                        line_number = format!("{:5} ", relative_distance).on(bg.clone()).with(line_color.clone());
                    }
                } else {
                    if current_line == row as i16 + buffer.scroll_offset.vertical as i16 + 1 {
                        line_number = format!("{:5} ", row as usize + buffer.scroll_offset.vertical + 1).on(Color::Reset).white();
                    } else {
                        line_number = format!("{:5} ", row as usize + buffer.scroll_offset.vertical + 1).on(Color::Reset).dark_grey();

                    }
                }
                line_number
            };
            let content = line_number.content();
            let style = line_number.style();
            let mut current_render_line = RenderLine { 
                cells: vec![
                    RenderCell { ch: " ".to_string(), style: ContentStyle::new().on(bg.clone()) };
                    self.size.cols as usize
                ]

            };
            let mut col = 0;
            for g in content.graphemes(true) {
                let width = UnicodeWidthStr::width(g) as usize;
                if col + width > self.size.cols as usize { break; }

                current_render_line.cells[col] = RenderCell { ch: g.to_string(), style: style.clone() };

                // fill any extra columns with blank placeholders to preserve spacing
                for i in 1..width {
                    if col + i < self.size.cols as usize {
                        current_render_line.cells[col + i] = RenderCell { ch: " ".to_string(), style: style.clone() };
                    }
                }

                col += width;
            }

            let styled_line = highlighter.highlight(line.unwrap().as_str(), row as usize + buffer.scroll_offset.vertical);
            for token in styled_line {
                let mut logical_col = token.offset; // where it *really* is in the file

                for g in token.text.graphemes(true) {
                    let width = UnicodeWidthStr::width(g) as usize;

                    // Skip until we reach the scrolled position
                    if logical_col + width <= buffer.scroll_offset.horizontal {
                        logical_col += width;
                        continue;
                    }

                    // Convert logical column to screen column
                    let screen_col = logical_col - buffer.scroll_offset.horizontal;

                    // Stop if out of screen
                    if screen_col + 6 >= self.size.cols as usize {
                        break;
                    }

                    let style = ContentStyle::new()
                        .on(bg.clone())
                        .with(token.style.unwrap_or(fg.clone()));

                    // Draw the first cell
                    if screen_col + 6 < self.size.cols as usize {
                        current_render_line.cells[screen_col + 6] =
                            RenderCell { ch: g.to_string(), style: style.clone() };
                    }

                    // Draw padding for full-width graphemes
                    for i in 1..width {
                        let sc = screen_col + i;
                        if sc + 6 < self.size.cols as usize {
                            current_render_line.cells[sc + 6] =
                                RenderCell { ch: " ".to_string(), style: style.clone() };
                        }
                    }

                    logical_col += width;
                }
            }


            self.render_buffer.current[row as usize + 1] = current_render_line;
        }

        Ok(())
    }
    */
}

impl Renderer for CrossTermRenderer {
    fn begin_frame(&mut self) {
        self.output.queue(terminal::BeginSynchronizedUpdate).expect("Could not begin synchronized update.");
        self.output.queue(cursor::Hide).expect("Could not hide cursor.");
    }

    fn draw_buffer(&mut self, editor: &Editor, ui: &UiManager, config: &Config) {
        let gutter_width = 5usize;
        let text_width = self.size.cols as usize - gutter_width;
        let height = self.size.rows as usize;

        let gutter_size = Size { cols: gutter_width as u16, rows: self.size.rows };
        let text_size   = Size { cols: text_width  as u16, rows: self.size.rows };

        let gutter = GutterLayer::render(editor, ui, config, gutter_size);
        let text   = TextLayer::render(editor, ui, config, text_size);

        let final_frame = Composite::merge(&gutter, &text);

        self.draw_frame(final_frame, config);
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
