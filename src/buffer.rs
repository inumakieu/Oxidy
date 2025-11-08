use crate::types::Size;

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
            if self.cursor.col == line.len() - 1 { return }

            self.cursor.col += 1;
        }
    }

    pub fn checked_row(&self) -> Option<usize> {
        return self.cursor.row.checked_sub(self.scroll_offset);
    }
}
