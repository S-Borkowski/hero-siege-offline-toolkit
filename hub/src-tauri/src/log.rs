//! A log file, because a panel that fails silently costs an evening.
//!
//! HS-Offline-Tracker learned this the hard way and routes everything the web
//! side throws into the app's log rather than a console nobody can see in a
//! released build. The hub does the same, and adds the install pipeline: a
//! refused hash or a declined UAC prompt should leave a line behind.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Log {
    path: PathBuf,
    gate: Mutex<()>,
}

/// Keep the file from growing without limit across a few hundred launches.
const MAX_BYTES: u64 = 1_000_000;

impl Log {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            gate: Mutex::new(()),
        }
    }

    pub fn write(&self, level: &str, message: &str) {
        let Ok(_guard) = self.gate.lock() else {
            return; // a poisoned lock is not worth failing a launch over
        };
        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0) > MAX_BYTES {
            // One generation back is plenty: the interesting failure is almost
            // always the most recent one.
            let _ = std::fs::rename(&self.path, self.path.with_extension("log.1"));
        }
        let line = format!(
            "{} [{}] {}\n",
            crate::state::now_iso(),
            level,
            message.replace('\n', "\n    ")
        );
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(line.as_bytes());
        }
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.write("info", message.as_ref());
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.write("error", message.as_ref());
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_appended_and_timestamped() {
        let dir = std::env::temp_dir().join(format!("hub-log-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = Log::new(dir.join("hub.log"));
        log.info("first");
        log.error("second");
        let text = std::fs::read_to_string(log.path()).unwrap();
        assert!(text.contains("[info] first"), "{text}");
        assert!(text.contains("[error] second"), "{text}");
        assert_eq!(text.lines().count(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_multi_line_message_stays_one_record() {
        let dir = std::env::temp_dir().join(format!("hub-log-multi-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = Log::new(dir.join("hub.log"));
        log.error("failed\nbecause of this");
        let text = std::fs::read_to_string(log.path()).unwrap();
        // Continuation lines are indented so a reader can tell them from new
        // records, and there is still exactly one timestamp.
        assert_eq!(text.matches("[error]").count(), 1);
        assert!(text.contains("\n    because of this"), "{text}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn writing_to_an_unwritable_path_does_not_panic() {
        // Logging must never be the thing that takes the app down.
        let log = Log::new(PathBuf::from("C:/\0invalid/hub.log"));
        log.info("this goes nowhere");
    }
}
