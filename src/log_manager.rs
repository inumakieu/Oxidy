use std::time::{Instant, Duration};

pub enum LogKind {
    Notification,
    Persistent,
}

pub struct TimedLog {
    pub time_created: Instant,
    pub duration: Duration,
    pub message: String,
}

pub struct LogManager {
    pub notifications: Vec<TimedLog>,
    pub persistent: Vec<String>,
}

impl LogManager {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            persistent: Vec::new()
        }
    }

    pub fn push_notification(&mut self, msg: String, dur: Duration) {
        self.notifications.push(TimedLog {
            message: msg,
            duration: dur,
            time_created: Instant::now(),
        });
    }

    pub fn push_persistent(&mut self, msg: String) {
        self.persistent.push(msg);
    }

    pub fn drain_notifications(&mut self) -> Vec<String> {
        let now = Instant::now();
        
        let mut new = Vec::new();
        self.notifications.retain(|log| {
            if now.duration_since(log.time_created) < log.duration {
                new.push(log.message.clone());
                true
            } else {
                false 
            }
        });
        
        new
    }

    pub fn drain_persistent(&mut self) -> Vec<String> {
        let msgs = self.persistent.clone();
        self.persistent.clear();
        msgs
    }
}

