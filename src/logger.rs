use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::SystemTime;
use chrono::{DateTime, Local};

/// Log levels to categorize the importance of messages
#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

/// The message structure that will be sent across the channel.
/// Using a struct allows us to pass metadata (like level and timestamp) 
/// to both the console and a future GUI.
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub message: String,
    pub timestamp: SystemTime,
}

pub struct Logger {
    sender: Sender<LogMessage>,
}

impl Logger {
    /// Initializes the logger, starts the background consumer thread, 
    /// and returns the Logger handle.
    pub fn new() -> Self {
        let (tx, rx): (Sender<LogMessage>, Receiver<LogMessage>) = mpsc::channel();

        // Spawn the background consumer thread
        thread::spawn(move || {
            Self::consumer_loop(rx);
        });

        Logger { sender: tx }
    }

    /// The consumer loop is the only place where actual output happens.
    /// This makes it easy to add a GUI hook or file logging later.
    fn consumer_loop(rx: Receiver<LogMessage>) {
        while let Ok(log) = rx.recv() {
            let level_str = match log.level {
                LogLevel::Info => "INFO",
                LogLevel::Warn => "WARN",
                LogLevel::Error => "ERROR",
                LogLevel::Debug => "DEBUG",
            };

            // Convert SystemTime to chrono::DateTime<Local>
            let datetime: DateTime<Local> = log.timestamp.into(); 
            // Format it: [Year-Month-Day Hour:Minute:Second]
            let timestamp_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

            let formatted_msg = format!(
                "[{}] {} - {}",
                timestamp_str, level_str, log.message
            );

            // 1. Print to console (Current requirement)
            match log.level {
                LogLevel::Info => println!("{}", log.message),
                _ => println!("{}", formatted_msg)
            }

            // 2. FUTURE: This is where you would send the message to your GUI 
            // or write to a log file.
            // gui_bridge::send_to_gui(formatted_msg);
        }
    }

    pub fn info(&self, msg: impl Into<String>) {
        self.log(LogLevel::Info, msg);
    }

    pub fn warn(&self, msg: impl Into<String>) {
        self.log(LogLevel::Warn, msg);
    }

    pub fn error(&self, msg: impl Into<String>) {
        self.log(LogLevel::Error, msg);
    }

    pub fn debug(&self, msg: impl Into<String>) {
        self.log(LogLevel::Debug, msg);
    }

    fn log(&self, level: LogLevel, msg: impl Into<String>) {
        let log_entry = LogMessage {
            level,
            message: msg.into(), // This converts &str to String, or leaves String as String
            timestamp: SystemTime::now(),
        };

        // Send the log message to the channel. 
        let _ = self.sender.send(log_entry);
    }
}