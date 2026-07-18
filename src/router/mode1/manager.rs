//! Mode1列表数据管理
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use tokio::{
    sync::{RwLock, broadcast::Receiver},
    task::JoinSet,
};

use crate::args::Filter;

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItem {
    pub args: Arc<RawValue>,
    pub data: Arc<RawValue>,
}

type Mode1Fn = Arc<dyn Fn(&Filter) -> (Arc<RawValue>, Receiver<Arc<RawValue>>) + Send + Sync + 'static>;

pub struct Mode1Manager {
    inner: RwLock<Vec<Mode1Fn>>,
}

impl Default for Mode1Manager {
    fn default() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
        }
    }
}

impl Mode1Manager {
    pub async fn register(&self, func: Mode1Fn) {
        self.inner.write().await.push(func);
    }

    pub async fn execute(&self, filter: &Filter) -> Vec<ListItem> {
        let funcs: Vec<Mode1Fn> = self.inner.read().await.iter().map(Arc::clone).collect();
        let filter = Arc::new(filter.clone());
        let mut tasks = JoinSet::new();

        for func in funcs {
            let filter = Arc::clone(&filter);
            tasks.spawn(async move {
                let (args, mut recv) = func(&filter);
                ListItem {
                    args,
                    data: recv.recv().await.unwrap(),
                }
            });
        }
        tasks.join_all().await
    }
}
