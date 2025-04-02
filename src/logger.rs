use chrono::Local;
use log::{LevelFilter, Record};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, Once};

static INIT: Once = Once::new();
static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);

pub fn init() {
    INIT.call_once(|| {
        // create log directory
        let log_dir = match get_log_dir() {
            Some(dir) => dir,
            None => {
                eprintln!("Failed to get log directory");
                return;
            }
        };
        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory: {}", e);
            return;
        }

        // create log file
        let log_file = log_dir.join(format!("thrift-ls-{}.log", Local::now().format("%Y%m%d")));
        match File::create(&log_file) {
            Ok(file) => {
                if let Ok(mut guard) = LOG_FILE.lock() {
                    *guard = Some(file);
                } else {
                    eprintln!("Failed to lock log file");
                    return;
                }
            }
            Err(e) => {
                eprintln!("Failed to create log file: {}", e);
                return;
            }
        }

        // set log level based on build configuration
        let level = if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        };
        log::set_max_level(level);

        // set custom logger
        if let Err(e) = log::set_boxed_logger(Box::new(CustomLogger)) {
            eprintln!("Failed to set custom logger: {}", e);
            return;
        }
    });
}

fn get_log_dir() -> Option<PathBuf> {
    let mut dir = dirs::home_dir()?;
    dir.push(".thrift-ls");
    dir.push("logs");
    Some(dir)
}

struct CustomLogger;

impl log::Log for CustomLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= LevelFilter::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let level = record.level();
            let target = record.target();
            let args = record.args();
            let line = record.line().unwrap_or(0);

            let message = format!("[{}] {} [{}:{}] {}\n", timestamp, level, target, line, args);

            // write to file
            if let Ok(mut guard) = LOG_FILE.lock() {
                if let Some(file) = guard.as_mut() {
                    if let Err(e) = file.write_all(message.as_bytes()) {
                        eprintln!("Failed to write to log file: {}", e);
                    }
                    if let Err(e) = file.flush() {
                        eprintln!("Failed to flush log file: {}", e);
                    }
                }
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = LOG_FILE.lock() {
            if let Some(file) = &mut *guard {
                if let Err(e) = file.flush() {
                    eprintln!("Failed to flush log file: {}", e);
                }
            }
        }
    }
}
