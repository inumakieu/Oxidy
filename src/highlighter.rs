use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::types::Token;
use crossterm::style::Color;
use regex::Regex;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct Highlighter {
    pub rules: Arc<Mutex<HashMap<String, String>>>,
    pub colors: HashMap<String, Color>,
    pub cache: HashMap<u64, Vec<Token>>
}

impl Highlighter {
    pub fn new(rules: Arc<Mutex<HashMap<String, String>>>) -> Self {
        let mut colors: HashMap<String, Color> = HashMap::new(); 
        colors.insert("keywords".to_string(), Color::Red);
        colors.insert("comments".to_string(), Color::DarkGrey);
        colors.insert("literals".to_string(), Color::Yellow);
        colors.insert("functions".to_string(), Color::Green);

        // rules.push((Regex::new(r"\b(let|pub|impl|fn|use)\b").unwrap(), Color::Red));
        
        Self { rules, colors, cache: HashMap::new() }
    }

    pub fn hash_bytes_default_hasher(&self, data: &[u8]) -> u64 {
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        hasher.finish()
    }

    pub fn highlight(&mut self, line: &str) -> Vec<Token> {
        let mut tokens: Vec<Token> = Vec::new();

        let checksum = self.hash_bytes_default_hasher(line.as_bytes());

        if let Some(cached) = self.cache.get(&checksum) {
            tokens.extend(cached.iter().cloned());
            return tokens;
        }

        
        if line.is_empty() {
            return tokens;
        }

        let rules = self.rules.lock().unwrap();

        for (key, value) in rules.iter() {
            let re = Regex::new(&value).unwrap();
            
            re.captures_iter(line)
                .for_each(|cap| {
                    if let Some(cap) = cap.get(1) {
                        tokens.push(Token { text: cap.as_str().to_string(), offset: cap.start(), style: Some(self.colors[key].clone()) })

                    }
                });
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
                    Token { text: found.clone(), offset: index - (found.len() - 1), style: Some(Color::Blue) }
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
