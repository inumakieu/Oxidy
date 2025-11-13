use crate::types::Size;

pub enum BufferLocation {
    Top,
    Bottom,
    StartLine,
    EndLine,
    PreviousWord,
    NextWord
}

#[derive(Debug, Clone)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub scroll_offset: usize,
    pub size: Size,
    pub path: String
}

impl Buffer {
    pub fn new(size: Size) -> Self {
        Self {
            lines: Vec::new(),
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0,
            size,
            path: "".to_string()
        }
    }

    pub fn text(&self) -> String {
        return self.lines.join("\n");
    }
    
    pub fn set(&mut self, lines: Vec<String>, path: String) {
        self.lines = lines;
        self.path = path;
    }

    pub fn get_at(&self, row: usize) -> Option<String> {
        self.lines.get(row + self.scroll_offset).cloned()
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor.row + self.scroll_offset) {
            line.insert(self.cursor.col, c);
            self.move_right();
        }
    }

    pub fn delete_char(&mut self) {
        let line_index = self.cursor.row + self.scroll_offset;
        let mut new_col = self.cursor.col;
        let mut move_up = false;

        if self.cursor.col == 0 {
            if line_index > 0 {
                // split the slice to borrow both lines safely
                let (before, after) = self.lines.split_at_mut(line_index);
                let prev = &mut before[line_index - 1];
                let curr = &mut after[0];
                new_col = prev.clone().len();
                prev.push_str(curr);
                self.lines.remove(line_index);
                move_up = true;
            }
        } else if let Some(line) = self.lines.get_mut(line_index) {
            if self.cursor.col <= line.len() {
                line.remove(self.cursor.col - 1);
                new_col -= 1;
            }
        }
        
        self.cursor.col = new_col;
        if move_up { self.move_up(); }
    }

    pub fn insert_newline(&mut self) {
        if self.cursor.row >= self.lines.len() {
            return;
        }

        // Take ownership of the current line (no borrow remains)
        let line = self.lines.remove(self.cursor.row);

        if self.cursor.col < line.len() {
            let (first, second) = line.split_at(self.cursor.col);

            self.lines.insert(self.cursor.row, first.to_string());
            self.lines.insert(self.cursor.row + 1, second.to_string());
        } else {
            // cursor at end → insert empty line
            self.lines.insert(self.cursor.row, line);
            self.lines.insert(self.cursor.row + 1, String::new());
        }

        self.cursor.row += 1;
        self.cursor.col = 0;
    }

    pub fn insert_tab(&mut self, tab_size: &usize) {
        if let Some(line) = self.lines.get_mut(self.cursor.row) {
            line.insert_str(self.cursor.col, " ".repeat(*tab_size).as_str());
            self.cursor.col += *tab_size;
        }
    }

    pub fn clamp_cursor(&mut self) {
        if let Some(line) = self.lines.get(self.cursor.row) {
            self.cursor.col = self.cursor.col.clamp(0, line.len());
        }
        self.cursor.row = self.cursor.row.clamp(0, self.lines.len() - 1);
    }

    pub fn move_up(&mut self) {
        if self.cursor.row == 0 { return }

        self.cursor.row -= 1;

        if (self.cursor.row as i16) >= self.scroll_offset as i16 { return }

        self.scroll_offset -= 1;
    }

    pub fn move_down(&mut self) {
        if self.cursor.row == self.lines.len() - 1 { return }

        self.cursor.row += 1;

        if self.cursor.row < (self.size.rows as usize - 1) + self.scroll_offset { return }

        self.scroll_offset += 1;
    }

    pub fn move_left(&mut self) {
        if self.cursor.col == 0 { return }

        self.cursor.col -= 1;
    }

    pub fn move_right(&mut self) {
        if let Some(line) = self.lines.get(self.cursor.row + self.scroll_offset) {
            if self.cursor.col == line.len() { return }

            self.cursor.col += 1;
        }
    }

    pub fn jump_to(&mut self, loc: BufferLocation) {
        match loc {
            BufferLocation::Top => {
                self.cursor.row = 0;
                self.scroll_offset = 0;
                
            }
            BufferLocation::Bottom => {
                self.cursor.row = self.lines.len() - 1;
                self.scroll_offset = self.cursor.row - (self.size.rows as usize - 1);
            }
            BufferLocation::StartLine => {
                if let Some(line) = self.lines.get(self.cursor.row) {
                    if let Some(index) = self.get_first_non_whitespace_char_index(&line) {
                        self.cursor.col = index;
                        return;
                    }
                }

                self.cursor.col = 0;
            }
            BufferLocation::EndLine => {
                if let Some(line) = self.lines.get(self.cursor.row) {
                    self.cursor.col = line.len();
                }            
            }
            _ => {}
        }
    }

    fn get_first_non_whitespace_char_index(&self, s: &str) -> Option<usize> {
        for (index, c) in s.char_indices() {
            if !c.is_whitespace() {
                return Some(index);
            }
        }
        None // No non-whitespace character found
    }

    pub fn checked_row(&self) -> Option<usize> {
        return self.cursor.row.checked_sub(self.scroll_offset);
    }
}
