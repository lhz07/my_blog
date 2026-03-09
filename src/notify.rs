use std::io::Write;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

macro_rules! swriteln {
    ($dst:expr, $($arg:tt)*) => {{
        use std::fmt::Write;
        writeln!($dst, $($arg)*).expect("Writing to String cannot fail")
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Notice,
    Warning,
    Critical,
}

impl AsRef<str> for Level {
    fn as_ref(&self) -> &str {
        match self {
            Level::Info => "Info",
            Level::Notice => "Notice",
            Level::Warning => "Warning",
            Level::Critical => "Critical",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct NotifyV1 {
    pub level: Level,
    pub title: String,
    pub program: String,
    pub body: String,
}

impl NotifyV1 {
    pub const VERSION: &str = "NOTIFY/1";
    pub const LEVEL: &str = "Level";
    pub const TITLE: &str = "Title";
    pub const PROGRAM: &str = "Program";
    pub const BODY_LENGTH: &str = "Body-Length";
    pub fn write_to_string(&self) -> String {
        let mut str = String::new();
        swriteln!(str, "{}", Self::VERSION);
        swriteln!(str, "{}: {}", Self::TITLE, self.title);
        swriteln!(str, "{}: {}", Self::PROGRAM, self.program);
        swriteln!(str, "{}: {}", Self::LEVEL, self.level.as_ref());
        swriteln!(str, "{}: {}", Self::BODY_LENGTH, self.body.len());
        swriteln!(str, "\n{}", self.body);
        str
    }
    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> Result<PathBuf, io::Error> {
        // it is highly recommended to use uuid v4
        let file_name = uuid::Uuid::new_v4().to_string();
        let path = dir.as_ref().join(&file_name);
        let mut file = fs::File::create_new(&path)?;
        file.write_all(self.write_to_string().as_bytes())?;
        Ok(path)
    }
}
