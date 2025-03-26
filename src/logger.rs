use log::{Record, Level, Metadata, SetLoggerError};
use rusqlite::{Connection, params};
use std::sync::{Mutex};
use once_cell::sync::Lazy;
use chrono::Utc;

struct SQLiteLogger {
    conn: Mutex<Connection>,
}

impl log::Log for SQLiteLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let conn = self.conn.lock().unwrap();
            let timestamp = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO logs (timestamp, level, module, message) VALUES (?1, ?2, ?3, ?4)",
                params![
                    timestamp,
                    record.level().to_string(),
                    record.target(),
                    record.args().to_string()
                ],
            ).unwrap();
        }
    }

    fn flush(&self) {}
}

// Use `Lazy` for safe, lazy initialization
static LOGGER: Lazy<SQLiteLogger> = Lazy::new(|| SQLiteLogger {
    conn: Mutex::new(Connection::open("logs.db").unwrap()),
});

pub fn init_logger() -> Result<(), SetLoggerError> {
    let conn = LOGGER.conn.lock().unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            level TEXT NOT NULL,
            module TEXT NOT NULL,
            message TEXT NOT NULL
        )",
        [],
    ).unwrap();

    log::set_logger(&*LOGGER).map(|()| log::set_max_level(log::LevelFilter::Info))
}
