use std::io::{self, stdout, Stdout, Write, StdoutLock};

use crossterm::cursor::SetCursorStyle;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::style::{Color, ContentStyle, ResetColor, SetStyle, StyledContent, Stylize};
use crossterm::{cursor::{self, MoveTo}, terminal, QueueableCommand};
use crossterm::{queue, ExecutableCommand};

use unicode_width::UnicodeWidthStr;
use unicode_segmentation::UnicodeSegmentation;

use crate::highlighter::Highlighter;
use crate::plugin_manager::Config;
use crate::renderer::Renderer;
use crate::buffer::Buffer;
use crate::types::{EditorMode, RenderBuffer, RenderCell, RenderLine, Size};
use crate::ui::command::Command;
use crate::ui::ui_manager::UiManager;
pub struct CrossTermRenderer {
    pub size: Size,
    pub render_buffer: RenderBuffer,
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
            render_buffer: RenderBuffer { 
                drawn: vec![
                    RenderLine { cells: Vec::new() }
                    ; size.rows as usize
                ], 
                current: vec![
                    RenderLine { cells: Vec::new() }
                    ; size.rows as usize
                ] 
            }, 
            output: output,
        }
    }

    fn line_visually_changed(&self, a: &RenderLine, b: &RenderLine) -> bool {
        if a.cells.len() != b.cells.len() {
            return true;
        }

        for (c1, c2) in a.cells.iter().zip(b.cells.iter()) {
            if c1.ch != c2.ch {
                return true;
            }
            if c1.style != c2.style {
                return true;
            }
        }

        false
    }
    
    fn redraw_line(&self, output: &mut StdoutLock, render_line: &RenderLine) {
        let mut current_style: Option<ContentStyle> = None;
        let mut col: u16 = 0;

        for cell in &render_line.cells {
            if current_style != Some(cell.style) {
                let _ = queue!(output, SetStyle(cell.style));
                current_style = Some(cell.style);
            }

            let _ = write!(output, "{}", cell.ch);

            col += cell.ch.width() as u16;
        }

        while col < self.size.cols {
            let _ = write!(output, " ");
            col += 1;
        }

        let _ = queue!(output, ResetColor);
    }

    fn textfield(&mut self, buffer: &Buffer, highlighter: &mut Highlighter, config: &Config) -> io::Result<()> {
        for row in 0..(self.size.rows - 1) {
            let line = buffer.get_at(row as usize);
            let mut current_render_line = RenderLine { 
                cells: vec![
                    RenderCell { ch: " ".to_string(), style: ContentStyle::new().reset() };
                    self.size.cols as usize
                ]
            };
            if line.is_none() {
                let empty = "    ∼ ".to_string().on(Color::Reset).dark_grey();
                for (index, char) in empty.content().chars().enumerate() {
                    current_render_line.cells[index] = RenderCell { ch: char.to_string(), style: empty.style().clone() };
                }
                self.render_buffer.current[row as usize] = current_render_line;
                continue;
            }

            let line_number = {
                let current_line = buffer.cursor.row as i16 + 1;
                let line_number: StyledContent<String>;
                if config.opt.relative_numbers {
                    let signed_row = row as i16 + 1;
                    let signed_scroll_offset = buffer.scroll_offset.vertical as i16;
                    let relative_distance = (current_line - (signed_row + signed_scroll_offset)).abs();
                    if current_line == signed_row + signed_scroll_offset { 
                        line_number = format!("{:5} ", current_line).reset();
                    } else {
                        line_number = format!("{:5} ", relative_distance).on(Color::Reset).dark_grey();
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
                    RenderCell { ch: " ".to_string(), style: ContentStyle::new().reset() };
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
                        .on(Color::Reset)
                        .with(token.style.unwrap_or(Color::Rgb { r: 230, g: 225, b: 233 }));

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
}

impl Renderer for CrossTermRenderer {
    fn begin_frame(&mut self) {
        self.output.queue(terminal::BeginSynchronizedUpdate).expect("Could not begin synchronized update.");
        self.output.queue(cursor::Hide).expect("Could not hide cursor.");
    }

    fn draw_buffer(&mut self, buffer: &mut Buffer, ui: &UiManager, highlighter: &mut Highlighter, editor_mode: &EditorMode, config: &Config) {
        let mut output = self.output.lock();
        queue!(output, MoveTo(0, 0)).expect("Could not move cursor to 0, 0.");

        let empty_line = RenderLine {
                cells: vec![
                    RenderCell { ch: " ".to_string(), style: ContentStyle::new().reset() };
                    self.size.cols as usize
                ]
        };
        self.render_buffer.current = vec![empty_line; self.size.rows as usize];

        let _ = self.textfield(buffer, highlighter, config);

        ui.render(&mut self.render_buffer.current);

        if self.render_buffer.current.len() == 0 {
            return;
        }

        for (index, current_line) in self.render_buffer.current.iter().enumerate() {
            let current_line = current_line.clone();

            if let Some(drawn_line) = self.render_buffer.drawn.get(index) {
                if self.line_visually_changed(drawn_line, &current_line) {
                    self.redraw_line(&mut output, &current_line);
                    self.render_buffer.drawn[index] = current_line.clone();
                }
            } else {
                self.redraw_line(&mut output, &current_line);
                self.render_buffer.drawn[index] = current_line.clone();
            }

            // only print newline if not last
            if index + 1 != self.render_buffer.current.len() {
                let _ = write!(output, "\r\n");
            }
        }
        // current -> drawn
        // self.render_buffer.drawn = self.render_buffer.current.clone();
        
        let checked_row = buffer.checked_row();

        if *editor_mode == EditorMode::NORMAL {
            let _ = self.output.queue(SetCursorStyle::BlinkingBlock);
        } else {
            let _ = self.output.queue(SetCursorStyle::BlinkingBar);
        }
          
        buffer.clamp_cursor();
        match editor_mode {
            EditorMode::INSERT |
            EditorMode::NORMAL => {
                if let Some(checked_row) = checked_row {
                    let _ = self.output.queue(MoveTo(6 + (buffer.cursor.col - buffer.scroll_offset.horizontal) as u16, checked_row as u16 + 1));
                } else {
                    let _ = self.output.queue(MoveTo(6 + (buffer.cursor.col - buffer.scroll_offset.horizontal) as u16, 1));
                }  
            }
            EditorMode::COMMAND => {
                let command = ui.get::<Command>();

                if let Some(command) = command {
                    let _ = self.output.queue(MoveTo(command.get_position() as u16, 1));
                }
            }
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
