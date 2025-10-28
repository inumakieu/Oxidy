use std::{fs::{self, read_to_string}, io::{self, stdout, Read, Stdout, Write}, process::exit, thread::sleep, time::Duration};
use crossterm::{
    cursor::{self, SetCursorStyle},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{self, style, Color, PrintStyledContent, ResetColor, SetBackgroundColor, SetForegroundColor, Stylize}, 
    terminal, ExecutableCommand, QueueableCommand
};
use rhai::{Dynamic, Engine, Map, Scope};
use std::fs::File;
use std::env;
use std::collections::HashMap;
use regex::Regex;

pub struct Size {
    pub cols: u16,
    pub rows: u16
}

#[derive(PartialEq)]
pub enum EditorMode {
    INSERT,
    COMMAND,
    NORMAL
}

#[derive(PartialEq)]
pub enum EditorEvent {
    EXIT,
    SAVE,
    CHANGE_MODE(EditorMode),
    NONE
}

#[derive(PartialEq)]
pub struct Location {
    pub col: u16,
    pub row: u16
}

pub struct Highlighter {
    pub rules: Vec<(Regex, Color)>
}

pub struct Token {
    pub text: String,
    pub offset: usize,
    pub style: Option<Color>
}

impl Highlighter {
    pub fn new() -> Self {
        let mut rules: Vec<(Regex, Color)> = Vec::new();
        
        rules.push((Regex::new(r"\blet \b").unwrap(), Color::Red));
        rules.push((Regex::new(r"\bpub \b").unwrap(), Color::Red));
        rules.push((Regex::new(r"\bimpl \b").unwrap(), Color::Red));
        rules.push((Regex::new(r"\bfn \b").unwrap(), Color::Red));
        rules.push((Regex::new(r"\buse \b").unwrap(), Color::Red));

        Self { rules }
    }

    pub fn highlight(&self, line: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        if line.is_empty() {
            return tokens;
        }

        for (regex, color ) in &self.rules {
            regex
                .find_iter(line)
                .for_each(|mat| {
                    tokens.push(Token { text: mat.as_str().to_string(), offset: mat.start(), style: Some(color.clone()) })
                });
        }

        let mut found: String = "".to_string();
        let mut found_tokens: Vec<Token> = Vec::new();
        
        // 0 - 3 -> "    "
        // 4 - 6 -> "pub"
        // 7 - 21 -> " struct Token {"
        let mut index = 0;
        while index <= line.len() - 1 {
            if let Some(token) = tokens.iter().find(|token| token.offset == index) {
                if !found.is_empty() {
                    found_tokens.push(
                        Token { text: found.clone(), offset: index - found.len(), style: Some(Color::Blue) }
                    );
                    found = "".to_string();
                }
                index += token.text.len();
                continue;
            }
            found.push(line.chars().nth(index).unwrap());

            if index == line.len() - 1 {
                found_tokens.push(
                    Token { text: found.clone(), offset: index - (found.len() - 1), style: Some(Color::Blue) }
                );
                found = "".to_string();
            }

            index += 1;
        } 
        
        tokens.extend(found_tokens);

        tokens.sort_by_key(|t| t.offset);

        tokens
    }
}

pub struct Editor {
    pub mode: EditorMode,
    pub output: Stdout,
    pub size: Size,
    pub command: String,
    pub text: Vec<String>,
    pub location: Location,
    pub current_path: String,
    pub scroll_offset: u16,
    pub highlighter: Highlighter
}

impl Editor {
    pub fn new() -> Self {
        let mut output = stdout();
        output.execute(terminal::EnterAlternateScreen).expect("Could not enter Alternate Screen.");
        terminal::enable_raw_mode().expect("Could not enable raw mode.");

        let term_size = terminal::size().expect("Size could not be determined.");

        let size = Size { cols: term_size.0, rows: term_size.1 };

        Self {
            mode: EditorMode::NORMAL,
            output,
            size,
            command: "".to_string(),
            text: Vec::new(),
            location: Location { col: 6, row: 0 },
            current_path: "".to_string(),
            scroll_offset: 0,
            highlighter: Highlighter::new()
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
            self.output.queue(terminal::BeginSynchronizedUpdate)?;
            self.output.queue(terminal::Clear(terminal::ClearType::All))?;
            self.output.queue(cursor::MoveTo(0,0))?;
            
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
                                    self.output.queue(cursor::SetCursorStyle::BlinkingBar)?;
                                } else {
                                    self.output.queue(cursor::SetCursorStyle::BlinkingBlock)?;
                                }
                            }
                            EditorEvent::EXIT => {
                                self.output.queue(terminal::EndSynchronizedUpdate)?;
                                self.output.flush()?;

                                return Ok(())
                            }
                            EditorEvent::SAVE => {
                                let content = self.text.join("\n");
                                fs::write(self.current_path.clone(), content)?;
                                self.command = "".to_string();
                            }
                            EditorEvent::NONE => {}
                        }
                    }
                    _ => {}
                }
            }
            
            if self.mode == EditorMode::COMMAND {
                self.output.queue(cursor::MoveTo(0, self.size.rows - 1))?;
                print!(":{}", self.command);
            } else {
                // self.output.queue(cursor::MoveTo(0, self.size.rows - 1))?; 
                // print!("--INSERT--");
                if let Some(current_line) = self.text.get(self.location.row as usize) {
                    self.location.col = self.location.col.clamp(6, 6 + current_line.len() as u16);
                    self.output.queue(cursor::MoveTo(self.location.col, self.location.row - self.scroll_offset))?;
                } else {
                    self.output.queue(cursor::MoveTo(6, 0))?;
                }
            }

            self.output.queue(terminal::EndSynchronizedUpdate)?;
            self.output.flush()?;

            // std::thread::sleep(Duration::from_millis(33));
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
                            _ => {}
                        }
                    }
                    EditorMode::COMMAND => self.command.push(char),
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
                
                let mut command_offset: u16 = 2;
                if self.mode == EditorMode::COMMAND { command_offset = 3; }

                // 61 + 34
                // scroll offset: 24
                // location row: 23
                if (self.location.row as i16) < self.scroll_offset as i16 {
                    /*
                    if self.location.row >= self.size.rows - command_offset - self.scroll_offset {
                        self.scroll_offset = self.location.row - (self.size.rows - command_offset);
                    }
                    */
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
                            self.location.row -= 1;
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
                    self.text.insert(self.location.row as usize + 1, "".to_string());
                    self.location.row += 1;
                    self.location.col = 6;
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
            self.output
                .queue(cursor::MoveTo(0, row))?;
            if let Some(line) = self.text.get(row as usize + self.scroll_offset as usize) {
                let current_line = self.location.row as i16 + 1;
                let signed_row = row as i16 + 1;
                let signed_scroll_offset = self.scroll_offset as i16;
                let relative_distance = (current_line - (signed_row + signed_scroll_offset)).abs();
                if current_line == signed_row + signed_scroll_offset { 
                    print!("{:5} ", current_line);
                } else {
                    self.output
                        .queue(
                            style::PrintStyledContent(
                                format!("{:5} ", relative_distance).dark_grey()
                            )
                        )?;
                }

                // TODO: Add regex highlighting
                let styled_line = self.highlighter.highlight(line);
                for token in styled_line {
                    self.output.queue(style::PrintStyledContent(token.text.with(token.style.unwrap_or(Color::White))))?;
                }
                // print!("{}", line);
            } else {
                self.output.queue(style::PrintStyledContent("    ∼ ".grey()))?; 
            }
        }

        Ok(())
    }

    pub fn status_bar(&mut self) -> io::Result<()> {
        let mut command_offset: u16 = 1;
        if self.mode == EditorMode::COMMAND { command_offset = 2; }
        self.output
            .queue(SetForegroundColor(style::Color::Black))?
            .queue(SetBackgroundColor(style::Color::White))?
            .queue(cursor::MoveTo(0, self.size.rows - command_offset))?;

        print!(" Oxidy ");
        
        self.output.queue(ResetColor)?
            .queue(SetForegroundColor(style::Color::White))?;
        print!("");

        let mut mode_text = "";
        match self.mode {
            EditorMode::INSERT => mode_text = "INS ",
            EditorMode::NORMAL => mode_text = "",
            EditorMode::COMMAND => mode_text = "CMD "
        }

        let right_bar = format!(" {:02}:{:02} {}", self.location.col - 5, self.location.row + 1, mode_text);
        let offset: u16 = right_bar.len() as u16;
        self.output
            .queue(cursor::MoveTo(self.size.cols - offset - 1, self.size.rows - command_offset))?;
        print!("");

        self.output
            .queue(SetForegroundColor(style::Color::Black))?
            .queue(SetBackgroundColor(style::Color::White))?;
        

        print!("{}", right_bar);

        self.output.queue(ResetColor)?;
        Ok(())
    }
}


impl Drop for Editor {
    fn drop(&mut self) {
        terminal::disable_raw_mode().expect("Could not disable raw mode.");
        self.output.execute(terminal::LeaveAlternateScreen).expect("Could not leave alternate screen.");
    }
}


fn create_config_object() -> rhai::Map {
    let mut conf_obj = Map::new();
    let mut opts_obj = Map::new();

    opts_obj.insert("relativenumbers".into(), false.into());

    conf_obj.insert("opt".into(), Dynamic::from(opts_obj));

    conf_obj
}

// TODO: Turn into PluginManager struct
fn plugin_loading() -> io::Result<()> {
    let engine = Engine::new(); 

    let mut config = File::open("/home/inumaki/.config/oxidy/config.rhai")?;
    let mut config_string = String::new();
    config.read_to_string(&mut config_string)?;

    let ast = engine.compile(&config_string).expect("AST creation failed.");

    let mut scope = Scope::new();

    scope.set_value("oxidy", create_config_object());

    let _ = engine.eval_ast_with_scope::<()>(&mut scope, &ast);

    let value: bool = engine.eval_with_scope(&mut scope, "oxidy.opt.relativenumbers").unwrap_or_default();
    println!("{:?}", value);
    
    Ok(())
}

fn main() -> io::Result<()> {
    let mut args = env::args();
    args.next();

    let mut editor = Editor::new();
   
    if let Some(input_file) = args.next() {
        editor.load_file(&input_file)?;
    }
    editor.run()?;
 
    Ok(())
}
