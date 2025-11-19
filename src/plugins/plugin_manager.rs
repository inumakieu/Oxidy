use std::{
    fs::{write, File}, io::{self, Read, Result}, path::PathBuf, sync::mpsc::{self, Receiver}, thread
};
use std::sync::{Arc, Mutex};
use crossterm::style::Color;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use rhai::{module_resolvers::FileModuleResolver, serde::{from_dynamic, to_dynamic}, Dynamic, Engine, FnPtr, NativeCallContext, Scope};

use std::collections::HashMap;

use crate::buffer::Buffer;
use crate::plugins::config::Config;
use crate::plugins::theme::Theme;

pub struct PluginManager {
    pub engine: Engine,
    pub config: Config,
    pub config_path: PathBuf,
    pub ast: rhai::AST,
    pub syntax: Arc<Mutex<HashMap<String, HashMap<String, String>>>>,
    pub current_lang: Arc<Mutex<Option<String>>>,

    pub rx: Option<Receiver<Event>>,
    // pub themes: Arc<Mutex<HashMap<String, HashMap<String, Color>>>>,
    // pub current_theme: Arc<Mutex<Option<String>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let config = Config::default();

        let mut config_path = dirs::home_dir().expect("Could not find home directory.");
        config_path.push(".config/oxidy/config.rhai");

        let config_file = File::open(&config_path);
        let mut engine = Engine::new();

        
        let mut resolver = FileModuleResolver::new();
        let mut base = dirs::home_dir().unwrap();
        base.push(".config/oxidy/");
        resolver.set_base_path(base); // or your ~/.config/oxidy
        engine.set_module_resolver(resolver);
        // engine.enable_imports(true);
        
        
        let ret: Self;
        let current_lang: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
                
        if let Ok(mut config_file) = config_file {
            let mut config_string = String::new();
            config_file.read_to_string(&mut config_string).unwrap();
 
            let ast = engine.compile(&config_string).expect("AST creation failed.");

            ret = Self {
                engine,
                ast,
                config,
                config_path,
                syntax: Arc::new(Mutex::new(HashMap::new())),
                current_lang,
                rx: None,
                // themes,
                // current_theme
            }
        } else {
            let ast = engine.compile("").unwrap();
            ret = Self {
                engine,
                ast,
                config,
                config_path,
                syntax: Arc::new(Mutex::new(HashMap::new())),
                current_lang,
                rx: None,
                // themes,
                // current_theme
            }
        }

        ret
    }

    /// Spawns a background thread that watches the config file
    pub fn start_watcher(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel::<Event>();
        let mut config_path = self.config_path.clone();
        
        config_path.pop();

        if !config_path.try_exists().unwrap_or(false) {
            return Ok(())
        }

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
        let mut config_path = self.config_path.clone();
        
        config_path.pop();

        if !config_path.try_exists().unwrap_or(false) {
            return 
        }

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
        let config_file = File::open(&self.config_path);

        match config_file {
            Ok(mut config_file) => {
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
            Err(_error) => {}
        }
    }

    pub fn load_config(&mut self) {
        let mut scope = Scope::new();
        let oxidy_config_struct = to_dynamic(self.config.clone()).unwrap();
        scope.set_value("oxidy", oxidy_config_struct);
        
        self.syntax();
        
        let _ = self.engine.eval_ast_with_scope::<()>(&mut scope, &self.ast);

        let script_result: Dynamic = self.engine.eval_with_scope(&mut scope, "oxidy").unwrap();
        let conf: Config = from_dynamic(&script_result).unwrap();

        self.config = conf.merge(&self.config);
    }

    pub fn get_current_theme_colors(&self) -> Option<HashMap<String, Color>> {
        let themes = self.config.themes.clone();
        let current_theme = self.config.theme.clone().unwrap();
        if let Some(colors) = themes.get(&current_theme) {
            let merged = colors.merge(&Theme::default());
            return Some(merged.to_map())
        }

        None
    }

    fn syntax(&mut self) {
        {
            let syntax_map = self.syntax.clone();         // Arc<Mutex<HashMap<String, HashMap<String, String>>>>
            let current_lang = self.current_lang.clone(); // Arc<Mutex<Option<String>>>
            self.engine.register_fn("set_syntax", move |key: String, value: String| {
                // read lang, then drop the lock
                let lang = {
                    let guard = current_lang.lock().unwrap();
                    guard.clone() // Option<String>
                }.expect("set_syntax called outside of a syntax(...) block");

                // write into syntax[lang][key] = value, creating the maps if needed
                let mut all = syntax_map.lock().unwrap();
                all.entry(lang).or_default().insert(key, value);
            });
        }

        {
            let current_lang = self.current_lang.clone(); 

            self.engine.register_fn("syntax", move |ctx: NativeCallContext, name: &str, callback: FnPtr| {
                {
                    let mut slot = current_lang.lock().unwrap();
                    *slot = Some(name.to_string());
                }
                let _: () = callback.call_within_context(
                    &ctx, 
                    ()
                ).unwrap();
            });
        }
    }

    pub fn save_buffer(&self, buffer: &Buffer) -> io::Result<()> {
        let content = buffer.lines.join("\n");
        write(buffer.path.clone(), content)
    }
}

