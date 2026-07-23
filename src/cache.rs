use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use serde_json::value::RawValue;
use tempfile::Builder;
use tokio::sync::broadcast::{self, Receiver};

#[derive(Clone)]
pub struct Cache {
    inner: Arc<CacheInner>,
}

struct CacheInner {
    directory: PathBuf,                                         // 缓存目录
    running: Mutex<HashMap<Arc<str>, Receiver<Arc<RawValue>>>>, // 任务队列
}

impl Cache {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(CacheInner {
                directory: directory.into(),
                running: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// 基于根目录创建子目录缓存，目录不存在则自动创建。
    pub fn sub(root: &std::path::Path, name: &str) -> io::Result<Self> {
        let dir = root.join(name);
        fs::create_dir_all(&dir)?;
        Ok(Self::new(dir))
    }

    /// 清空缓存目录（包括所有缓存文件）。
    pub fn clear(&self) -> io::Result<()> {
        let dir = &self.inner.directory;
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(())
    }
    pub fn get(&self, args: &str) -> Option<Receiver<Arc<RawValue>>> {
        let receiver = {
            let running = self.inner.running.lock().unwrap();
            running.get(args).map(Receiver::resubscribe)
        };

        if let Some(receiver) = receiver {
            return Some(receiver);
        }

        let (tx, rx) = broadcast::channel(1);
        let file_path = self.inner.directory.join(format!("{args}.json"));
        let json = fs::read_to_string(file_path).ok()?;
        let value = RawValue::from_string(json).ok().map(Arc::from)?;
        tx.send(value).ok()?;
        Some(rx)
    }

    /// 内部使用spawn_blocking执行可以跑耗时任务
    pub fn get_or_run<F>(&self, args: Arc<str>, func: F) -> Receiver<Arc<RawValue>>
    where
        F: FnOnce() -> Box<RawValue> + Send + 'static,
    {
        if let Some(json) = self.get(&args) {
            return json;
        }

        self.run(args, func)
    }

    pub fn run(&self, args: Arc<str>, func: impl FnOnce() -> Box<RawValue> + Send + 'static) -> Receiver<Arc<RawValue>> {
        let mut running = self.inner.running.lock().unwrap();
        if let Some(rx) = running.get(args.as_ref()) {
            return rx.resubscribe();
        }

        let (tx, rx) = broadcast::channel(1);
        running.insert(args.clone(), rx.resubscribe());
        drop(running);

        let cache = self.clone();
        tokio::task::spawn_blocking(move || {
            let result = catch_unwind(AssertUnwindSafe(|| Arc::<RawValue>::from(func())));

            match result {
                Ok(json) => {
                    if let Err(err) = cache.save(&args, &json) {
                        eprintln!("保存缓存 {args} 失败: {err}");
                    }

                    let mut running = cache.inner.running.lock().unwrap();
                    let _ = tx.send(json);
                    running.remove(&args);
                }
                Err(payload) => {
                    cache.inner.running.lock().unwrap().remove(&args);
                    resume_unwind(payload);
                }
            }
        });

        rx
    }

    fn save(&self, args: &str, json: &RawValue) -> io::Result<()> {
        let directory = &self.inner.directory;
        fs::create_dir_all(directory)?;

        let file_path = directory.join(format!("{args}.json"));
        let mut temp_file = Builder::new().suffix(".tmp").tempfile_in(directory)?;
        temp_file.write_all(json.get().as_bytes())?;
        temp_file.as_file().sync_all()?;
        temp_file.persist(file_path).map_err(|err| err.error)?;

        Ok(())
    }
}
