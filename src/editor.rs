#![allow(non_snake_case)]

use std::io::{self, Read};
use std::sync::Arc;
use std::fs::File;

use crate::buffer::Buffer;
use crate::input::{EditorCommand, InputHandler};

use crate::plugin_manager::PluginManager;
use crate::renderer::Renderer;
use crate::services::lsp_service::{LspService, LspServiceEvent};
use crate::types::{EditorEvent, EditorMode, Size};
use crate::highlighter::Highlighter;
use crate::ui::command::Command;
use crate::ui::status_bar::StatusBar;
use crate::ui::ui_manager::UiManager;


pub struct Editor {
    pub mode: EditorMode,

    pub buffer: Buffer, // TODO: Make multi buffer

    pub command: String,

    pub ui: UiManager,
    pub renderer: Box<dyn Renderer>,
    pub input: Box<dyn InputHandler>,
    pub plugins: PluginManager,
    pub highlighter: Highlighter,
    pub lsp: Option<LspService>
}

impl Editor {
    pub fn new(size: Size, renderer: Box<dyn Renderer>, input: Box<dyn InputHandler>) -> Self { 
        let mut plugins = PluginManager::new();
        plugins.load_config();
        plugins.start_watcher().unwrap();
        
        let highlighter = Highlighter::new(
            Arc::clone(&plugins.syntax)
        );

        let mut ui = UiManager::new();
        let status_bar = StatusBar::new();
        let command = Command::new();
        ui.add(status_bar);
        ui.add(command);

        let lsp = LspService::new();

        Self {
            mode: EditorMode::NORMAL,
            buffer: Buffer::new(size),
            command: "".to_string(),
            ui,
            renderer,
            input,
            plugins,
            highlighter,
            lsp
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
        
        self.buffer.set(lines.clone(), path.to_string());
        
        let file_type_index = path.to_string().rfind(".");
        if let Some(file_type_index) = file_type_index {
            let file_type = &path[file_type_index + 1..];

            self.highlighter.init(file_type.to_string());
        }

        let status = self.ui.get_mut::<StatusBar>();

        if let Some(status) = status {
            status.file = path.to_string().clone();
        }

        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {        
        loop {
            self.plugins.poll_reload();
            if let Some(lsp) = self.lsp.as_mut() {
                match lsp.poll() {
                    LspServiceEvent::Initialized => {
                        let buffer_clone = self.buffer.clone();
                        lsp.open_file(&buffer_clone.path, &buffer_clone.text());
                    }
                    LspServiceEvent::OpenedFile => {
                        lsp.request_semantic_tokens(&self.buffer);
                    }
                    LspServiceEvent::ReceivedSemantics { semantics: _ } => {
                        let colors = self.plugins.get_current_theme_colors().unwrap_or(self.highlighter.colors.clone());
                        self.highlighter.tokens = lsp.set_tokens(&self.buffer, colors);
                        self.highlighter.cache.clear();
                        
                    }
                    _ => {}
                }
            }
            if let Some(cmd) = self.input.poll(&self.mode)? {
                if self.handle_command(cmd)? == EditorEvent::Exit {
                    break;
                }
            }
            let status = self.ui.get_mut::<StatusBar>();

            if let Some(status) = status {
                status.pos = self.buffer.cursor.clone();
                status.mode = self.mode.clone();
            }

            self.renderer.begin_frame();
            self.renderer.draw_buffer(&mut self.buffer, &self.ui, &mut self.highlighter, &self.mode, &self.plugins.config);
            self.renderer.end_frame();
        }

        Ok(())
    }

    fn handle_command(&mut self, cmd: EditorCommand) -> io::Result<EditorEvent> {
        match cmd {
            EditorCommand::MoveUp => self.buffer.move_up(),
            EditorCommand::MoveDown => self.buffer.move_down(),
            EditorCommand::MoveLeft => self.buffer.move_left(),
            EditorCommand::MoveRight => self.buffer.move_right(),
            EditorCommand::JumpTo(loc) => self.buffer.jump_to(loc),
            EditorCommand::InsertChar(c) => self.buffer.insert_char(c),
            EditorCommand::InsertCommandChar(c) => {
                self.command.push(c);
                let command = self.ui.get_mut::<Command>();

                if let Some(command) = command {
                    command.command = self.command.clone();
                }
            }
            EditorCommand::Tab => self.buffer.insert_tab(&self.plugins.config.opt.tab_size),
            EditorCommand::Enter => self.buffer.insert_newline(),
            EditorCommand::ChangeMode(mode) => {
                self.mode = mode;
                let command = self.ui.get_mut::<Command>();

                if let Some(command) = command {
                    command.shown = self.mode == EditorMode::COMMAND;
                }
            }
            EditorCommand::Backspace => {
                match self.mode {
                    EditorMode::INSERT => self.buffer.delete_char(),
                    EditorMode::COMMAND => {
                        self.command.pop();
                        let command = self.ui.get_mut::<Command>();

                        if let Some(command) = command {
                            command.command = self.command.clone();
                        }
                    }
                    _ => {}
                }
            }
            EditorCommand::LeaveMode => {
                self.mode = EditorMode::NORMAL;
                let command = self.ui.get_mut::<Command>();

                if let Some(command) = command {
                    command.shown = false;
                }

            }
            EditorCommand::RunCommand => {
                match self.command.as_str() {
                    "q" => return Ok(EditorEvent::Exit),
                    "w" => {
                        self.plugins.save_buffer(&self.buffer);
                        return Ok(EditorEvent::Save)
                    }
                    theme if theme.contains("theme ") => {
                        let split: Vec<&str> = theme.split_whitespace().collect();
                        let name = split[1];
                        self.plugins.config.theme = name.to_string();

                        let colors = self.plugins.get_current_theme_colors().unwrap_or(self.highlighter.colors.clone());
                        if let Some(lsp) = self.lsp.as_mut() {
                            self.highlighter.tokens = lsp.set_tokens(&self.buffer, colors);
                        }

                        self.highlighter.cache.clear();
                    }
                    "lsp" => {
                        let buffer_clone = self.buffer.clone();
                        let root_index = buffer_clone.path.rfind("/").unwrap();
                        let root_uri = &buffer_clone.path[0..root_index];
                        self.with_lsp(|lsp| {
                            lsp.initialize(&root_uri);
                        });
                    }, // TODO: Make it spawn lsp specified
                    _ => {}
                }

                self.command = "".to_string();
                let command = self.ui.get_mut::<Command>();

                if let Some(command) = command {
                    command.command = self.command.clone();
                }
            }
            EditorCommand::Save => self.plugins.save_buffer(&self.buffer),
            EditorCommand::Quit => return Ok(EditorEvent::Exit),
            _ => {}
        }
        Ok(EditorEvent::None)
    }

    fn with_lsp<F>(&mut self, f: F)
    where
        F: FnOnce(&mut LspService)
    {
        if let Some(lsp) = self.lsp.as_mut() {
            f(lsp);
        }
    }

    /*
    pub fn render(&mut self) -> io::Result<()> {
        loop {
            self.plugin_manager.poll_reload();

            let empty_line = RenderLine {
                    cells: vec![
                        RenderCell { ch: " ".to_string(), style: ContentStyle::new().reset() };
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
                                if self.plugin_manager.config.opt.natural_scroll {
                                    self.move_cursor_left();
                                } else {
                                    self.move_cursor_right()
                                }
                            }
                            MouseEventKind::ScrollLeft => {
                                if self.plugin_manager.config.opt.natural_scroll {
                                    self.move_cursor_right();
                                } else {
                                    self.move_cursor_left()
                                }
                            }
                            MouseEventKind::Down(button) => {
                                if button.is_left() {
                                    let new_col = mouse_event.column;
                                    let new_row = (mouse_event.row + self.scroll_offset) - 1;

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
                        RenderCell { ch: char.to_string(), style: command_input.style().clone() }
                    );
                }
                self.render_buffer.current[self.size.rows as usize - 1] = render_line;
            }

            // TEMP
            self.render_cards()?;
            
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
                        self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
                        let checked_row = self.location.row.checked_sub(self.scroll_offset);
                        if let Some(checked_row) = checked_row {
                            self.output.queue(MoveTo(self.location.col, checked_row + 1))?;
                        } else {
                            self.output.queue(MoveTo(self.location.col, 1))?;
                        }
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

    pub fn move_cursor_down(&mut self) {
        if self.location.row < self.text.len() as u16 - 1 {
            self.location.row += 1;
        }
        self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);
                
        if self.location.row >= self.size.rows + self.scroll_offset {
            self.scroll_offset += 1;
        }
        self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
    }

    pub fn move_cursor_up(&mut self) {
        if self.location.row > 0 {
            self.location.row -= 1;
        }
        self.location.row = self.location.row.clamp(0, self.text.len() as u16 - 1);

        if (self.location.row as i16) < self.scroll_offset as i16 {            
            self.scroll_offset -= 1;
        }
        self.scroll_offset = self.scroll_offset.clamp(0, self.text.len() as u16 - 1);
    }

    pub fn move_cursor_left(&mut self) {
        if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
            self.location.col -= 1;
            self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16);
        } 
    }

    pub fn move_cursor_right(&mut self) {
        if let Some(current_line) = self.text.get_mut(self.location.row as usize) {
            self.location.col += 1;
            self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16 + 6);
        }
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
                self.move_cursor_up();
            }
            KeyCode::Down => {
                self.move_cursor_down();
            }
            KeyCode::Left => {
                self.move_cursor_left();
            }
            KeyCode::Right => {
                self.move_cursor_right();
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
        for row in 0..(self.size.rows - 1) {
            let line = self.text.get(row as usize + self.scroll_offset as usize);
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

            let styled_line = self.highlighter.highlight(line.unwrap(), row as usize + self.scroll_offset as usize);
            for token in styled_line {
                let mut col = 6 + token.offset; // still okay if offset is character-based
                for g in token.text.graphemes(true) {
                    let width = UnicodeWidthStr::width(g) as usize;
                    if col >= self.size.cols as usize { break; }

                    let style = ContentStyle::new()
                        .on(Color::Reset)
                        .with(token.style.unwrap_or(Color::Rgb { r: 230, g: 225, b: 233 }));

                    current_render_line.cells[col] = RenderCell { ch: g.to_string(), style: style.clone() };

                    for i in 1..width {
                        if col + i < self.size.cols as usize {
                            current_render_line.cells[col + i] = RenderCell { ch: " ".to_string(), style: style.clone() };
                        }
                    }

                    col += width;
                }
            }

            self.render_buffer.current[row as usize + 1] = current_render_line;
        }

        Ok(())
    } 

    pub fn status_bar(&mut self) -> io::Result<()> {
        let mut render_line = RenderLine { cells: Vec::new() };

        // TODO: Add file path
        let bg = Color::Rgb { r: 32, g: 31, b: 37 };
        let fg = Color::Rgb { r: 230, g: 225, b: 233 };
        let left_bar = format!(" Oxidy ").bold().on(bg.clone()).with(fg.clone());
        let right_symbol = "".to_string().on(Color::Reset).with(bg.clone());
        let left_symbol = "".to_string().on(Color::Reset).with(bg.clone());

        let mut file_name = " empty ".to_string();
        if let Some(file_name_index) = self.current_path.rfind("/") {
            file_name = format!(" {} ", self.current_path[file_name_index + 1..].to_string());
        }
        let file_name_bar = file_name.black().on(bg.clone()).with(fg.clone());

        let mode_text: &str;
        match self.mode {
            EditorMode::INSERT => mode_text = "INS ",
            EditorMode::NORMAL => mode_text = "",
            EditorMode::COMMAND => mode_text = "CMD "
        }

        let right_bar = format!(" {:02}:{:02} {}", self.location.col - 5, self.location.row + 1, mode_text).on(bg.clone()).with(fg.clone());
        
        let spacing = " ".to_string().reset();

        let gap: u16 = self.size.cols - 1 - (
            1 + left_bar.content().len() + 1 + 1 +
            1 + file_name_bar.content().len() + 1 +
            1 + right_bar.content().len() + 1) as u16;
        let gap_content = StyledContent::new(ContentStyle::new().reset(), format!("{}", " ".repeat(gap as usize)));

        let status_bar = vec![
            left_symbol.clone(), left_bar, right_symbol.clone(),
            spacing.clone(),
            left_symbol.clone(), file_name_bar, right_symbol.clone(),
            gap_content, 
            left_symbol.clone(), right_bar, right_symbol.clone()];
        for item in status_bar {
            for char in item.content().chars() {
                render_line.cells.push(
                    RenderCell { ch: char.to_string(), style: item.style().clone() }
                );
            }
        }
        
        self.render_buffer.current[0] = render_line;
        
        Ok(())
    }

    pub fn render_cards(&mut self) -> io::Result<()> {
        let cards = self.cards.clone();
        for card in cards.iter() {
            self.render_card(card)?;
        }

        Ok(())
    }

    pub fn render_card(&mut self, card: &Card) -> io::Result<()> {
        let top_left = '╭';
        let top_right = '╮';
        let bottom_left = '╰';
        let bottom_right = '╯';

        let horizontal = '─';
        let vertical = '│';

        let max_width = 63;
        let max_height = 12;
        let padding = 1;
        
        let lines = card.get_lines(max_width - 2 - (padding * 2));
        let width = max_width;
        let height = (lines.len() + 2).clamp(3, max_height);
        let offset = self.size.cols as usize - width - 1;
        let style = card.card_type.style();

        for y in 0..height {
            let mut render_line = self.render_buffer.current[self.size.rows as usize - (height - y)].clone();            
            let mut char: char;
            for x in 0..width {
                if y == 0 {
                    if x == 0 {
                        char = top_left.clone();
                    } else if x == width - 1 {
                        char = top_right.clone();
                    } else {
                        char = horizontal.clone();
                    }
                } else if y == height - 1 {
                    if x == 0 {
                        char = bottom_left.clone();
                    } else if x == width - 1 {
                        char = bottom_right.clone();
                    } else {
                        char = horizontal.clone();
                    }
                } else {
                    if x == 0 || x == width - 1 {
                        char = vertical.clone();
                    } else if x <= padding || x >= width - 1 - padding {
                        char = ' ';
                    } else { 
                        let mut chars = lines[y - 1].chars();
                        char = chars.nth(x - 1 - padding).unwrap_or(' ');
                    }
                }
                render_line.cells[x + offset] = RenderCell { ch: char.to_string(), style: style };

            }
            self.render_buffer.current[self.size.rows as usize - (height - y)] = render_line
        }

        Ok(())
    }

    pub fn cleanup(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode.");
        self.output.execute(terminal::LeaveAlternateScreen).expect("Could not leave alternate screen.");
        self.output.execute(Show).expect("Could not show cursor.");
        self.output.execute(DisableMouseCapture).expect("Could not disable mouse capture.");
    }
    */
}


