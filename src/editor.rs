use std::io::{self, stdout, Read, Stdout, StdoutLock, Write};
use std::sync::Arc;
use std::time::Duration;
use std::fs::{File, write};

use crossterm::cursor::{self, MoveTo, SetCursorStyle, Show};
use crossterm::event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use crossterm::style::{self, Color, ContentStyle, PrintStyledContent, ResetColor, SetBackgroundColor, SetForegroundColor, SetStyle, StyledContent, Stylize}; 
use crossterm::{terminal, ExecutableCommand, QueueableCommand};
use crossterm::queue;

use crate::plugin_manager::PluginManager;
use crate::types::{EditorEvent, EditorMode, Location, RenderBuffer, RenderCell, RenderLine, Size};
use crate::highlighter::Highlighter;

pub struct Editor {
    pub mode: EditorMode,
    pub output: Stdout,
    pub size: Size,
    pub command: String,
    
    pub render_buffer: RenderBuffer,
    pub text: Vec<String>,
    
    pub location: Location,
    pub current_path: String,
    pub scroll_offset: u16,

    pub highlighter: Highlighter,
    pub plugin_manager: PluginManager
}

impl Editor {
    pub fn new() -> Self {
        let mut output = stdout();
        output.execute(terminal::EnterAlternateScreen).expect("Could not enter Alternate Screen.");
        terminal::enable_raw_mode().expect("Could not enable raw mode.");
        output.execute(EnableMouseCapture).expect("Could not enable mouse capture.");

        let term_size = terminal::size().expect("Size could not be determined.");

        let size = Size { cols: term_size.0, rows: term_size.1 };

        let mut plugin_manager = PluginManager::new();
        plugin_manager.load_config();
        plugin_manager.start_watcher().unwrap();

        let highlighter = Highlighter::new(Arc::clone(&plugin_manager.syntax));

        Self {
            mode: EditorMode::NORMAL,
            output,
            size,
            command: "".to_string(),
            render_buffer: RenderBuffer { drawn: Vec::new(), current: Vec::new() },
            text: Vec::new(),
            location: Location { col: 6, row: 0 },
            current_path: "".to_string(),
            scroll_offset: 0,
            highlighter,
            plugin_manager
        }
    }

    pub fn load_file(&mut self, path: &str) -> io::Result<()> {
        let mut file = File::open(path)?;
        
        let mut file_string = String::new();
        file.read_to_string(&mut file_string)?;

        // convert string to Vec<String>
        let lines: Vec<String> = file_string
            .split("\n")
            .map(|s| s.to_string())
            .collect();

        self.text = lines;
        self.current_path = path.to_string();
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.render()?;
        
        Ok(())
    }

    pub fn render(&mut self) -> io::Result<()> {
        loop {
            self.plugin_manager.poll_reload();

            let empty_line = RenderLine {
                    cells: vec![
                        RenderCell { ch: ' ', style: ContentStyle::new().reset() };
                        self.size.cols as usize
                    ]
            };
            self.render_buffer.current = vec![empty_line; self.size.rows as usize];
            self.output.queue(terminal::BeginSynchronizedUpdate)?;
            self.output.queue(cursor::Hide)?;
            let mut output = self.output.lock();
            queue!(output, MoveTo(0, 0))?;
            // self.output.queue(terminal::Clear(terminal::ClearType::All))?;
            // self.output.queue(MoveTo(0,0))?;
            
            self.textfield()?;
            self.status_bar()?;

            
            if poll(Duration::from_millis(0))? { 
                match read()? {
                    Event::Key(event) => { 
                        let editor_event = self.handle_input(event)?;
                        
                        match editor_event {
                            EditorEvent::CHANGE_MODE(mode) => {
                                self.mode = mode;
                                if self.mode == EditorMode::INSERT {
                                    self.output.queue(SetCursorStyle::BlinkingBar)?;
                                } else {
                                    self.output.queue(SetCursorStyle::BlinkingBlock)?;
                                }
                            }
                            EditorEvent::EXIT => {
                                self.output.queue(terminal::EndSynchronizedUpdate)?;
                                self.output.flush()?;

                                return Ok(())
                            }
                            EditorEvent::SAVE => {
                                let content = self.text.join("\n");
                                write(self.current_path.clone(), content)?;
                                self.command = "".to_string();
                            }
                            EditorEvent::NONE => {}
                        }
                    }
                    Event::Mouse(mouse_event) => {
                        if self.text.is_empty() { continue; };

                        let scroll_up_kind = {
                            let kind: MouseEventKind;
                            if self.plugin_manager.config.opt.natural_scroll { kind = MouseEventKind::ScrollDown; }
                            else { kind = MouseEventKind::ScrollUp }

                            kind
                        };

                        match mouse_event.kind {
                            MouseEventKind::ScrollDown => {
                                if self.plugin_manager.config.opt.natural_scroll { 
                                    self.move_cursor_up()
                                } else { 
                                    self.move_cursor_down()
                                }
                            }
                            MouseEventKind::ScrollUp => {
                                if self.plugin_manager.config.opt.natural_scroll { 
                                    self.move_cursor_down()
                                } else { 
                                    self.move_cursor_up()
                                }
                            }
                            MouseEventKind::ScrollRight => {
                                if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                                    self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16)    
                                }

                                self.location.col -= 1;
                                self.location.col = self.location.col.clamp(6, self.size.cols as u16);
                            }
                            MouseEventKind::ScrollLeft => {
                                if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                                    self.location.col += 1;
                                    self.location.col = self.location.col.clamp(6, current_line.len() as u16 + 6);
                                }
                            }
                            MouseEventKind::Down(button) => {
                                if button.is_left() {
                                    let new_col = mouse_event.column;
                                    let new_row = mouse_event.row + self.scroll_offset;

                                    self.location.col = new_col;
                                    self.location.row = new_row;
                                }
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(new_w, new_h) => {
                        self.size = Size { cols: new_w, rows: new_h };
                    }
                    _ => {}
                }
            }
            
            
            if self.mode == EditorMode::COMMAND {
                let command_input = format!(":{}", self.command).reset().white();
                let mut render_line = RenderLine { cells: Vec::new() };

                for char in command_input.content().chars() {
                    render_line.cells.push(
                        RenderCell { ch: char, style: command_input.style().clone() }
                    );
                }
                self.render_buffer.current[self.size.rows as usize - 1] = render_line;
            }
            
            // diff render_buffer 
            if self.render_buffer.current.len() == 0 {
                continue;
            }

            for (index, current_line) in self.render_buffer.current.iter().enumerate() {
                let current_line = current_line.clone();

                if let Some(drawn_line) = self.render_buffer.drawn.get(index) {
                    if *drawn_line != current_line {
                        self.redraw_line(&mut output, &current_line);
                    }
                } else {
                    self.redraw_line(&mut output, &current_line);
                }

                // only print newline if not last
                if index + 1 != self.render_buffer.current.len() {
                    let _ = write!(output, "\r\n");
                }
            }
            // current -> drawn
            self.render_buffer.drawn = self.render_buffer.current.clone();

            match self.mode {
                EditorMode::INSERT | EditorMode::NORMAL => {
                    if let Some(current_line) = self.text.get(self.location.row as usize) {
                        self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16);
                        self.output.queue(MoveTo(self.location.col, self.location.row - self.scroll_offset))?;
                    } else {
                        self.output.queue(MoveTo(6, 0))?;
                    }
                }
                EditorMode::COMMAND => {
                    self.output.queue(MoveTo(1 + self.command.len() as u16, self.size.rows - 1))?;
                }
            }
            
            self.output.queue(cursor::Show)?;
            self.output.queue(terminal::EndSynchronizedUpdate)?;
            self.output.flush()?;
        }
    }
    
    pub fn redraw_line(&self, output: &mut StdoutLock, render_line: &RenderLine) {
        let mut current_style: Option<ContentStyle> = None;
        for col in 0..self.size.cols - 1 {// render_line.cells.clone() {
            if let Some(cell) = render_line.cells.get(col as usize) {
                if current_style == None || cell.style != current_style.unwrap() {
                    let _ = queue!(output, SetStyle(cell.style));
                    current_style = Some(cell.style);
                }
                let _ = write!(output, "{}", cell.ch);

                continue;
            }
            let _ = queue!(output, ResetColor);
            let _ = write!(output, " ");
        }
    }

    pub fn move_cursor_down(&mut self) {
        if self.location.row < self.text.len() as u16 - 1 {
            self.location.row += 1;
        }
        self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
        
        let mut command_offset: u16 = 1;
        if self.mode == EditorMode::COMMAND { command_offset = 2; }

        if self.location.row >= self.size.rows - command_offset + self.scroll_offset {
            self.scroll_offset += 1;
        }
        self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
    }

    pub fn move_cursor_up(&mut self) {
        if self.location.row > 0 {
            self.location.row -= 1;
        }
        self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
        
        let mut command_offset: u16 = 1;
        if self.mode == EditorMode::COMMAND { command_offset = 2; }

        if self.location.row >= self.size.rows - command_offset + self.scroll_offset {            
            self.scroll_offset -= 1;
        }
        self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
    }


    pub fn handle_input(&mut self, event: KeyEvent) -> io::Result<EditorEvent> {
        match event.code {
            KeyCode::Char(char) => {
                match self.mode {
                    EditorMode::NORMAL => {
                        match char {
                            'i' => return Ok(EditorEvent::CHANGE_MODE(EditorMode::INSERT)),
                            ':' => return Ok(EditorEvent::CHANGE_MODE(EditorMode::COMMAND)),
                            'd' => {
                                self.render_buffer.dump_to_file("/home/inumaki/.config/oxidy/current_buffer.txt")?;
                            }
                            _ => {}
                        }
                    }
                    EditorMode::COMMAND => {
                        self.command.push(char);
                        match char {
                            'd' => {
                                self.render_buffer.dump_to_file("/home/inumaki/.config/oxidy/current_buffer.txt")?;
                            }
                            _ => {}
                        }
                    }
                    EditorMode::INSERT => {
                        if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                            self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16);
                            current_line.insert(self.location.col as usize - 6, char);
                            self.location.col += 1;
                        } else {
                            self.text.push(char.to_string());
                        }
                    }
                }
            } 
            KeyCode::Tab => {
                if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                    self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16);
                    current_line.insert_str(self.location.col as usize - 6, "    ");
                    self.location.col += 4;
                } else {
                    self.text.push("    ".to_string());
                }
            }
            KeyCode::Up => {
                if self.location.row > 0 {
                    self.location.row -= 1;
                }
                self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
                
                if (self.location.row as i16) < self.scroll_offset as i16 {
                    self.scroll_offset -= 1;
                }
                self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
            }
            KeyCode::Down => {
                if self.location.row < self.text.len() as u16 - 1 {
                    self.location.row += 1;
                }
                self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
                
                let mut command_offset: u16 = 1;
                if self.mode == EditorMode::COMMAND { command_offset = 2; }

                if self.location.row >= self.size.rows - command_offset + self.scroll_offset {
                    self.scroll_offset += 1;
                }
                self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
            }
            KeyCode::Left => {
                if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                    self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16)    
                }

                self.location.col -= 1;
                self.location.col = self.location.col.clamp(6, self.size.cols as u16);
            }
            KeyCode::Right => {
                if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                    self.location.col += 1;
                    self.location.col = self.location.col.clamp(6, current_line.len() as u16 + 6);
                }
            }
            KeyCode::Backspace => {
                if self.mode == EditorMode::INSERT {
                    if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
                        if self.location.col == 6 {
                            self.text.remove(self.location.row as usize);
                            if self.location.row > 0 {
                                self.location.row -= 1;
                            }
                            if let Some(new_row) = self.text.get(self.location.row as usize) {
                                self.location.col = new_row.len() as u16 + 6;
                            }
                        } else {
                            current_line.remove(self.location.col as usize - 7);
                            self.location.col -= 1;
                            self.location.col = self.location.col.clamp(4, self.size.cols as u16);
                        }
                    }
                } else {
                    self.command.pop();
                }
            }
            KeyCode::Esc => {
                match self.mode {
                    EditorMode::INSERT => return Ok(EditorEvent::CHANGE_MODE(EditorMode::NORMAL)),
                    EditorMode::COMMAND => {
                        self.command.clear();
                        return Ok(EditorEvent::CHANGE_MODE(EditorMode::NORMAL))
                    }
                    EditorMode::NORMAL => self.command.clear(),
                }
            }
            KeyCode::Enter => {
                if self.mode == EditorMode::COMMAND {
                    match self.command.as_str() {
                        "q" => return Ok(EditorEvent::EXIT),
                        "w" => return Ok(EditorEvent::SAVE),
                        _ => {}
                    }
                }

                if self.mode == EditorMode::INSERT {
                    if self.location.row as usize + 1 >= self.text.len() {
                        self.text.push("".to_string());
                    } else {
                        self.text.insert(self.location.row as usize + 1, "".to_string());
                    }
                    self.location.row += 1;
                    self.location.col = 6;

                    let mut command_offset = 2;
                    if self.mode == EditorMode::COMMAND {command_offset = 3};
                    if self.location.row - self.scroll_offset > self.size.rows - command_offset {
                        self.scroll_offset += 1;
                    }
                }
            }
            _ => {}
        }

        Ok(EditorEvent::NONE)
    }

    pub fn textfield(&mut self) -> io::Result<()> {
        let mut command_offset: u16 = 1;
        if self.mode == EditorMode::COMMAND { command_offset = 2; }

        for row in 0..(self.size.rows - command_offset) {
            let line = self.text.get(row as usize + self.scroll_offset as usize);
            let mut current_render_line = RenderLine { cells: Vec::new() };
            if line.is_none() {
                let empty = "    ∼ ".to_string().on(Color::Reset).dark_grey();
                for char in empty.content().chars() {
                    current_render_line.cells.push(RenderCell { ch: char, style: empty.style().clone() });
                }
                self.render_buffer.current[row as usize] = current_render_line;
                continue;
            }

            let line_number = {
                let current_line = self.location.row as i16 + 1;
                let line_number: StyledContent<String>;
                if self.plugin_manager.config.opt.relative_numbers {
                    // TODO: Add relative numbers back
                    let signed_row = row as i16 + 1;
                    let signed_scroll_offset = self.scroll_offset as i16;
                    let relative_distance = (current_line - (signed_row + signed_scroll_offset)).abs();
                    if current_line == signed_row + signed_scroll_offset { 
                        line_number = format!("{:5} ", current_line).reset();
                    } else {
                        line_number = format!("{:5} ", relative_distance).on(Color::Reset).dark_grey();
                    }
                } else {
                    if current_line == row as i16 + self.scroll_offset as i16 + 1 {
                        line_number = format!("{:5} ", row + self.scroll_offset + 1).on(Color::Reset).white();
                    } else {
                        line_number = format!("{:5} ", row + self.scroll_offset + 1).on(Color::Reset).dark_grey();

                    }
                }
                line_number
            };
            let content = line_number.content();
            let style = line_number.style();
            let mut current_render_line = RenderLine { cells: Vec::new() };
            for char in content.chars() {
                current_render_line.cells.push(RenderCell { ch: char, style: style.clone() });
            }

            let styled_line = self.highlighter.highlight(line.unwrap());
            for token in styled_line {
                for char in token.text.chars() {
                    let text_style = ContentStyle::new()
                        .on(Color::Reset)
                        .with(token.style.unwrap_or(Color::White));
                    current_render_line.cells.push(RenderCell { ch: char, style: text_style });

                }
            }

            self.render_buffer.current[row as usize] = current_render_line;

            /*
            self.output
                .queue(MoveTo(0, row))?;
            if let Some(line) = self.text.get(row as usize + self.scroll_offset as usize) {
                let current_line = self.location.row as i16 + 1;
                
                if self.plugin_manager.config.opt.relativenumbers {
                    let signed_row = row as i16 + 1;
                    let signed_scroll_offset = self.scroll_offset as i16;
                    let relative_distance = (current_line - (signed_row + signed_scroll_offset)).abs();
                    if current_line == signed_row + signed_scroll_offset { 
                        print!("{:5} ", current_line);
                    } else {
                        self.output
                            .queue(
                                PrintStyledContent(
                                    format!("{:5} ", relative_distance).dark_grey()
                                )
                            )?;
                    }
                } else {
                    if current_line == row as i16 + self.scroll_offset as i16 + 1 { 
                        print!("{:5} ", current_line);
                    } else {
                        self.output
                            .queue(
                                PrintStyledContent(
                                    format!("{:5} ", row + self.scroll_offset + 1).dark_grey()
                                )
                            )?;
                    }
                }

                let styled_line = self.highlighter.highlight(line);
                for token in styled_line {
                     self.output.queue(PrintStyledContent(token.text.with(token.style.unwrap_or(Color::White))))?;
                }
            } else {
                self.output.queue(PrintStyledContent("    ∼ ".dark_grey()))?;
                
            }
            */
        }

        Ok(())
    } 

    pub fn status_bar(&mut self) -> io::Result<()> {
        let mut render_line = RenderLine { cells: Vec::new() };
        let mut command_offset = 1;
        if self.mode == EditorMode::COMMAND { command_offset = 2; }


        // TODO: Add file path
        let left_bar = format!(" Oxidy ").black().on_white();
        let left_chevron = "".to_string().reset().white(); 

        let mode_text: &str;
        match self.mode {
            EditorMode::INSERT => mode_text = "INS ",
            EditorMode::NORMAL => mode_text = "",
            EditorMode::COMMAND => mode_text = "CMD "
        }

        let right_bar = format!(" {:02}:{:02} {}", self.location.col - 5, self.location.row + 1, mode_text).black().on_white();
        let right_chevron = "".to_string().reset().white(); 

        let gap: u16 = self.size.cols - 1 - (left_bar.content().len() + 2 + right_bar.content().len()) as u16;
        let gap_content = StyledContent::new(ContentStyle::new().reset(), format!("{}", " ".repeat(gap as usize)));

        let status_bar = vec![left_bar, left_chevron, gap_content, right_chevron, right_bar];
        for item in status_bar {
            for char in item.content().chars() {
                render_line.cells.push(
                    RenderCell { ch: char, style: item.style().clone() }
                );
            }
        }
        
        self.render_buffer.current[self.size.rows as usize - command_offset] = render_line;
        
        Ok(())
    }

    pub fn cleanup(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode.");
        self.output.execute(terminal::LeaveAlternateScreen).expect("Could not leave alternate screen.");
        self.output.execute(Show).expect("Could not show cursor.");
        self.output.execute(DisableMouseCapture).expect("Could not disable mouse capture.");
    }
}


impl Drop for Editor {
    fn drop(&mut self) {
        self.cleanup();
    }
}
