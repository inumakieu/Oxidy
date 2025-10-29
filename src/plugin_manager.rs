use std::{
    fs::File,
    io::{Read, Result},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};
use std::sync::{Arc, Mutex};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rhai::{Engine, Scope, Dynamic, serde::{from_dynamic, to_dynamic}};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Options {
    pub relativenumbers: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub opt: Options,
}

pub struct PluginManager {
    pub engine: Engine,
    pub config: Config,
    pub config_path: PathBuf,
    pub ast: rhai::AST,
    pub syntax: Arc<Mutex<HashMap<String, String>>>,

    // Channel for file watch events
    pub rx: Option<Receiver<Event>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let config = Config {
            opt: Options {
                relativenumbers: false,
            },
        };

        let mut config_path = dirs::home_dir().expect("Could not find home directory.");
        config_path.push(".config/oxidy/config.rhai");

        let mut config_file = File::open(&config_path).expect("Config file not found.");
        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string).unwrap();

        let engine = Engine::new();
        let ast = engine.compile(&config_string).expect("AST creation failed.");

        Self {
            engine,
            ast,
            config,
            config_path,
            syntax: Arc::new(Mutex::new(HashMap::new())),
            rx: None,
        }
    }

    /// Spawns a background thread that watches the config file
    pub fn start_watcher(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Event>();
        let config_path = self.config_path.clone();

        // Spawn the watcher thread
        thread::spawn(move || {
            // clone the sender so the closure can capture it
            let tx_watch = tx.clone();

            // define how notify should deliver events
            let mut watcher = notify::recommended_watcher(move |res| {
                match res {
                    Ok(event) => {
                        let _ = tx_watch.send(event);
                    }
                    Err(e) => eprintln!("watch error: {:?}", e),
                }
            })
            .expect("Failed to create watcher");

            watcher
                .watch(&config_path, RecursiveMode::NonRecursive)
                .expect("Failed to watch config file.");
 
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        });

        self.rx = Some(rx);
        Ok(())
    }

    /// Checks if a reload event occurred (non-blocking)
    pub fn poll_reload(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok(event) = rx.try_recv() {
                // println!("Config file changed: {:?}", event);
                match event.kind {
                    EventKind::Modify(_) => self.reload_config(),
                    _ => {}
                }
            }
        }
    }

    /// Re-loads and re-evaluates the Rhai config
    pub fn reload_config(&mut self) {
        let mut config_file = File::open(&self.config_path).expect("Config file not found.");
        let mut config_string = String::new();
        config_file.read_to_string(&mut config_string).unwrap();

        match self.engine.compile(&config_string) {
            Ok(ast) => {
                self.ast = ast;
                self.load_config();
                // println!("Config reloaded successfully!");
            }
            Err(err) => {
                println!("Error reloading config: {:?}", err);
            }
        }
    }

    pub fn load_config(&mut self) {
        let mut scope = Scope::new();
        let oxidy_config_struct = to_dynamic(self.config.clone()).unwrap();
        scope.set_value("oxidy", oxidy_config_struct);

        // print!("Config");
        let syntax_map = self.syntax.clone();
        self.engine.register_fn("set_syntax", move |key: String, value: String| {
            let mut map = syntax_map.lock().unwrap(); 
            map.insert(key, value);
            // print!("syntax set.");
        });

        let _ = self.engine.eval_ast_with_scope::<()>(&mut scope, &self.ast);

        let script_result: Dynamic = self.engine.eval_with_scope(&mut scope, "oxidy").unwrap();
        self.config = from_dynamic(&script_result).unwrap();
    }
}

