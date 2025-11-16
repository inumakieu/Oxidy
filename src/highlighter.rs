use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::types::Token;
use crossterm::style::Color;
use regex::Regex;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Highlighter {
    pub current_filetype: String,
    pub rules: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    pub colors: HashMap<String, Color>,
    pub cache: HashMap<u64, Vec<Token>>,
    pub tokens: Vec<Vec<Token>>
}

impl Highlighter {
    pub fn new(rules: Arc<Mutex<HashMap<String, HashMap<String, String>>>>) -> Self {
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
            cache: HashMap::new(),
            tokens: Vec::new(),
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

    pub fn highlight(&mut self, line: &str, index: usize) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        if !self.tokens.is_empty() {
            let val =  self.tokens.get(index);
            match val {
                Some(val) => tokens.extend(val.clone()),
                None => {}
            }
        }

        let checksum = self.hash_bytes_default_hasher(line.as_bytes());

        if let Some(cached) = self.cache.get(&checksum) {
            tokens.extend(cached.iter().cloned());
            return tokens;
        }

        
        if line.is_empty() {
            return tokens;
        }

        if tokens.is_empty() {
            let syntax_map = self.rules.lock().unwrap();
            let rules = syntax_map.get(&self.current_filetype);
            if rules.is_none() {
                tokens.push(Token { text: line.to_string(), offset: 0, style: Some(self.colors["fg"].clone()) });
                return tokens
            }

            for (key, value) in rules.unwrap().iter() {
                let re = Regex::new(&value).unwrap();
                
                re.captures_iter(line)
                    .for_each(|cap| {
                        if let Some(cap) = cap.get(1) {
                            tokens.push(Token { text: cap.as_str().to_string(), offset: cap.start(), style: Some(self.colors[key].clone()) })
                        }
                    });
            }
        }

        let mut found: String = "".to_string();
        let mut found_tokens: Vec<Token> = Vec::new();
        
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
            if let Some(char) = line.chars().nth(index) {
                found.push(char);
            }

            if index == line.len() - 1 {
                found_tokens.push(
                    Token { text: found.clone(), offset: index - (found.len() - 1), style: Some(Color::White) }
                );
                found = "".to_string();
            }

            index += 1;
        } 
        
        tokens.extend(found_tokens);

        tokens.sort_by_key(|t| t.offset);

        self.cache.insert(checksum, tokens.clone());
        tokens
    }
}
