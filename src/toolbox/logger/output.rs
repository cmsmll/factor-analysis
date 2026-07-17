use std::{
    fs::{self, File},
    io,
    path::PathBuf,
};

use enum_dispatch::enum_dispatch;
use time::{Date, OffsetDateTime};

use crate::toolbox::logger::{message::Message, now};

#[enum_dispatch(OutputMethod)]
pub trait Output {
    fn output(&mut self, message: &Message) -> io::Result<()>;
}

/// 输出方式
#[enum_dispatch]
pub enum OutputMethod {
    Stdout(Stdout),
    OutputFile(OutFile),
}

/// 标准输出
#[derive(Debug)]
pub struct Stdout {
    pub color: bool,
    pub output: std::io::Stdout,
}

impl Output for Stdout {
    fn output(&mut self, message: &Message) -> io::Result<()> {
        if self.color {
            message.write_color(&mut self.output)
        } else {
            message.write(&mut self.output)
        }
    }
}

impl Default for Stdout {
    fn default() -> Self {
        Self {
            color: true,
            output: std::io::stdout(),
        }
    }
}

/// 文件输出
#[derive(Debug)]
pub struct OutFile {
    pub file: File,
    pub name: String,
    pub path: PathBuf,
    pub delete: Option<i64>,
    pub current_date: Date,
}

impl OutFile {
    pub fn new(path: PathBuf, name: String, delete: Option<i64>) -> io::Result<Self> {
        let current_time = now();
        fs::create_dir_all(&path)?;
        let file_name = name.replace("{date}", &current_time.date().to_string());
        let file = File::options().create(true).append(true).open(path.join(&file_name))?;
        Ok(Self {
            path,
            name,
            delete,
            current_date: current_time.date(),
            file,
        })
    }

    pub fn delete_log_file(&self, current_time: OffsetDateTime) -> io::Result<()> {
        if let Some(retention_days) = self.delete {
            for entry in fs::read_dir(&self.path)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }

                let meta = entry.metadata()?;
                let modified_at: OffsetDateTime = meta.modified()?.into();
                if (current_time - modified_at).whole_days() > retention_days {
                    fs::remove_file(entry.path())?;
                }
            }
        }
        Ok(())
    }

    /// 更新日志文件 删除过期文件
    pub fn update_log_file(&mut self, current_time: OffsetDateTime) -> io::Result<()> {
        if let Err(err) = self.delete_log_file(current_time) {
            eprintln!("日志删除失败: {err}");
        }
        let name = self.name.replace("{date}", &current_time.date().to_string());
        self.file = File::options().create(true).append(true).open(self.path.join(name))?;
        self.current_date = current_time.date();
        Ok(())
    }
}

impl Output for OutFile {
    fn output(&mut self, message: &Message) -> io::Result<()> {
        if message.begin.date() != self.current_date {
            self.update_log_file(message.begin)?;
        }
        message.write(&mut self.file)
    }
}
