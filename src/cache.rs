use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use bitcode::Encode;
use serde_json::value::RawValue;
use tempfile::Builder;
use tokio::sync::broadcast::{self, Receiver};

/// 哈希值
pub trait HashCode: Encode {
    fn hashcode(&self) -> Arc<str> {
        let buf = bitcode::encode(self);
        let res = blake3::hash(&buf);
        Arc::from(res.to_string())
    }
}

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
    pub async fn get(&self, args: &str) -> Option<Arc<RawValue>> {
        let receiver = {
            let running = self.inner.running.lock().unwrap();
            running.get(args).map(Receiver::resubscribe)
        };

        if let Some(mut receiver) = receiver {
            return receiver.recv().await.ok();
        }

        let file_path = self.inner.directory.join(format!("{args}.json"));
        let json = tokio::fs::read_to_string(file_path).await.ok()?;
        RawValue::from_string(json).ok().map(Arc::from)
    }

    pub async fn get_or_run(
        &self,
        args: Arc<str>,
        f: impl FnOnce() -> Box<RawValue> + Send + 'static,
    ) -> Result<Arc<RawValue>, broadcast::error::RecvError> {
        if let Some(json) = self.get(&args).await {
            return Ok(json);
        }

        self.run(args, f).await.recv().await
    }

    pub async fn run(
        &self,
        args: Arc<str>,
        f: impl FnOnce() -> Box<RawValue> + Send + 'static,
    ) -> Receiver<Arc<RawValue>> {
        let mut running = self.inner.running.lock().unwrap();
        if let Some(rx) = running.get(args.as_ref()) {
            return rx.resubscribe();
        }

        let (tx, rx) = broadcast::channel(1);
        running.insert(args.clone(), rx.resubscribe());
        drop(running);

        let cache = self.clone();
        tokio::task::spawn_blocking(move || {
            let result = catch_unwind(AssertUnwindSafe(|| Arc::<RawValue>::from(f())));

            match result {
                Ok(json) => {
                    if let Err(err) = Self::save(&cache.inner.directory, &args, &json) {
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

    fn save(directory: &Path, args: &str, json: &RawValue) -> io::Result<()> {
        fs::create_dir_all(directory)?;

        let file_path = directory.join(format!("{args}.json"));
        let mut temp_file = Builder::new().suffix(".tmp").tempfile_in(directory)?;
        temp_file.write_all(json.get().as_bytes())?;
        temp_file.as_file().sync_all()?;
        temp_file.persist(file_path).map_err(|err| err.error)?;

        Ok(())
    }
}
