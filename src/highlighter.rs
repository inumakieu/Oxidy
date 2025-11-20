use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;

use crate::types::Token;
use crossterm::style::Color;
use regex::Regex;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub struct Highlighter {
    pub current_filetype: String,
    pub rules: HashMap<String, HashMap<String, String>>,
    pub colors: HashMap<String, Color>,
    pub tokens: RefCell<Vec<Vec<Token>>>,
    pub cache: RefCell<HashMap<u64, Vec<Token>>>
}

impl Highlighter {
    pub fn new(rules: HashMap<String, HashMap<String, String>>) -> Self {
        let mut colors: HashMap<String, Color> = HashMap::new();

        colors.insert("bg".into(), Color::Reset);
        colors.insert("fg".into(), Color::White);

        colors.insert("namespace".into(), Color::Blue);
        colors.insert("type".into(), Color::Magenta);
        colors.insert("class".into(), Color::Magenta);
        colors.insert("enum".into(), Color::Magenta);
        colors.insert("interface".into(), Color::Magenta);
        colors.insert("struct".into(), Color::Magenta);
        colors.insert("typeParameter".into(), Color::Cyan);

        colors.insert("parameter".into(), Color::White);
        colors.insert("variable".into(), Color::White);
        colors.insert("property".into(), Color::Yellow);
        colors.insert("enumMember".into(), Color::Yellow);

        colors.insert("event".into(), Color::Green);
        colors.insert("function".into(), Color::Green);
        colors.insert("method".into(), Color::Green);
        colors.insert("macro".into(), Color::Cyan);

        colors.insert("keyword".into(), Color::Blue);
        colors.insert("modifier".into(), Color::Blue);
        colors.insert("operator".into(), Color::White);

        colors.insert("comment".into(), Color::DarkGrey);
        colors.insert("string".into(), Color::Red);
        colors.insert("number".into(), Color::Cyan);
        colors.insert("regexp".into(), Color::Cyan);

        Self {
            current_filetype: "".to_string(),
            rules,
            colors,
            cache: RefCell::new(HashMap::new()),
            tokens: RefCell::new(Vec::new()),
        }
    }

    pub fn init(&mut self, current_filetype: String) {
        self.current_filetype = current_filetype;
    }

    pub fn hash_bytes_default_hasher(&self, data: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }

    pub fn highlight(&self, line: &str, index: usize) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        if let Some(val) = self.tokens.borrow().get(index) {
            tokens.extend(val.clone());
        }

        let checksum = self.hash_bytes_default_hasher(line.as_bytes());

        if let Some(cached) = self.cache.borrow().get(&checksum) && cached.len() > 0 {
            tokens.extend(cached.clone());
            return tokens;
        }

        if line.is_empty() {
            return tokens;
        }

        if tokens.is_empty() {
            if let Some(rules) = self.rules.get(&self.current_filetype) {
                for (key, regex_source) in rules {
                    let re = Regex::new(regex_source).unwrap();

                    for cap in re.captures_iter(line) {
                        if let Some(cap) = cap.get(1) {
                            tokens.push(Token {
                                text: cap.as_str().to_string(),
                                offset: cap.start(),
                                style: Some(self.colors[key].clone()),
                            });
                        }
                    }
                }
            } else {
                tokens.push(Token {
                    text: line.to_string(),
                    offset: 0,
                    style: Some(self.colors["fg"].clone()),
                });
            }
        }

        let mut found_tokens = Vec::new();
        let mut buffer = String::new();

        let mut i = 0;
        while i < line.len() {
            let is_token_start = tokens.iter().any(|t| t.offset == i);

            if is_token_start {
                if !buffer.is_empty() {
                    let start = i - buffer.len();
                    found_tokens.push(Token {
                        text: buffer.clone(),
                        offset: start,
                        style: Some(Color::White),
                    });
                    buffer.clear();
                }

                if let Some(existing) = tokens.iter().find(|t| t.offset == i) {
                    i += existing.text.len();
                    continue;
                }
            }

            if let Some(ch) = line.chars().nth(i) {
                buffer.push(ch);
            }

            if i == line.len() - 1 && !buffer.is_empty() {
                let start = i + 1 - buffer.len();
                found_tokens.push(Token {
                    text: buffer.clone(),
                    offset: start,
                    style: Some(Color::White),
                });
            }

            i += 1;
        }

        tokens.extend(found_tokens);
        tokens.sort_by_key(|t| t.offset);

        self.cache.borrow_mut().insert(checksum, tokens.clone());

        tokens
    }

    pub fn shift_line_tokens(&self, row: usize, col: usize, width: isize) {
        if let Some(tokens) = self.tokens.borrow_mut().get_mut(row) {
            for token in tokens {
                if token.offset >= col {
                    let new_offset = (token.offset as isize) + width;
                    token.offset = new_offset.max(0) as usize;
                }
            }
        }
        self.cache.borrow_mut().clear();
    }

    pub fn get_tokens(&self, row: usize) -> Option<Vec<Token>> {
        let value = self.tokens.borrow();

        return value.clone().get(row).cloned()
    }
    
    pub fn update_tokens(&self, tokens: Vec<Vec<Token>>) {
        *self.tokens.borrow_mut() = tokens;
        self.cache.borrow_mut().clear();
    }
}
