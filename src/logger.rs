use std::fs::{File, OpenOptions};
use std::sync::Mutex;
use std::io::Write;
use std::sync::OnceLock;

pub struct Logger {
    file: Mutex<File>
}

impl Logger {
    pub fn new() -> Self {
        let w = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/oxidy.log")
            .unwrap();

        Self {
            file: Mutex::new(w)
        }
    }

    pub fn log(&self, message: String) {
        let mut f = self.file.lock().unwrap();
        writeln!(f, "{}", message).unwrap();
    }
}

pub static LOGGER: OnceLock<Logger> = OnceLock::new();