pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

pub struct Buffer {
    pub lines: Vec<String>,
    pub cursor: Cursor,
    pub scroll_offset: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            cursor: Cursor { row: 0, col: 0 },
            scroll_offset: 0
        }
    }
    
    pub fn set_lines(&mut self, lines: Vec<String>) {
        self.lines = lines;
    }

    pub fn get_at(&self, row: usize) -> Option<String> {
        self.lines.get(row + self.scroll_offset).cloned()
    }

    pub fn insert_char(&mut self, c: char) {
        if let Some(line) = self.lines.get_mut(self.cursor.row + self.scroll_offset) {
            line.insert(self.cursor.col, c);
        }
    }

    pub fn delete_char(&mut self) {
        if let Some(line) = self.lines.get_mut(self.cursor.row + self.scroll_offset) {
            line.remove(self.cursor.col);
        }
    }

    pub fn insert_newline(&mut self) {
        if let Some(line) = self.lines.get(self.cursor.row + self.scroll_offset) {
            // check if inside a line
            if self.cursor.col < line.len() - 1 {
                let splits = line.split_at(self.cursor.col);
                self.lines.insert(self.cursor.row + self.scroll_offset + 1, splits.1.to_string());
                return
            }

            // if not, just insert new empty line
            self.lines.insert(self.cursor.row + self.scroll_offset + 1, "".to_string());
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.row == 0 { return }

        self.cursor.row -= 1;
    }

    pub fn move_down(&mut self) {
        if self.cursor.row == self.lines.len() - 1 { return }

        self.cursor.row += 1;
    }

    pub fn move_left(&mut self) {
        if self.cursor.col == 0 { return }

        self.cursor.col -= 1;
    }

    pub fn move_right(&mut self) {
        if let Some(line) = self.lines.get(self.cursor.row + self.scroll_offset) {
            if self.cursor.col == line.len() - 1 { return }

            self.cursor.col += 1;
        }
    }
}
