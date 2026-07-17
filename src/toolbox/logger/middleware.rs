use std::{
    sync::Arc,
    thread::{self, JoinHandle},
    time::Instant,
};

use crossbeam_channel::Sender;
use percent_encoding::percent_decode;
use salvo::{Depot, FlowCtrl, Handler, Request, Response, async_trait, conn::SocketAddr, http::header::LOCATION};

use crate::toolbox::logger::{
    message::Message,
    now,
    output::{OutFile, Output, OutputMethod, Stdout},
};

#[derive(Clone)]
pub struct Logger {
    sender: Sender<Message>,
    #[allow(unused)]
    handle: Arc<JoinHandle<()>>,
}

impl Logger {
    pub fn new(mut writers: Vec<OutputMethod>) -> Self {
        let (sender, rx) = crossbeam_channel::unbounded::<Message>();

        let handle = thread::Builder::new()
            .name("salvo-logger".into())
            .spawn(move || {
                for msg in rx {
                    for writer in writers.iter_mut() {
                        if let Err(err) = writer.output(&msg) {
                            eprintln!("输出日志失败: {err}");
                        }
                    }
                }
            })
            .expect("failed to spawn logger thread")
            .into();

        Self { sender, handle }
    }
}

impl Default for Logger {
    fn default() -> Self {
        let mut writers = vec![OutputMethod::Stdout(Stdout::default())];

        match OutFile::new("logs".into(), "{date}.log".into(), None) {
            Ok(file) => writers.push(OutputMethod::OutputFile(file)),
            Err(err) => eprintln!("初始化日志文件失败，将只输出到终端: {err}"),
        }

        Self::new(writers)
    }
}

#[async_trait]
impl Handler for Logger {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        let begin = now();
        let started_at = Instant::now();
        let method = req.method().to_string();
        let ip = match req.remote_addr() {
            SocketAddr::IPv4(ip) => ip.ip().to_string(),
            SocketAddr::IPv6(ip) => ip.ip().to_string(),
            #[cfg(unix)]
            #[cfg_attr(docsrs, doc(cfg(unix)))]
            SocketAddr::Unix(ip) => match ip.as_pathname() {
                Some(path) => path.display().to_string(),
                None => "Unknown".to_string(),
            },
            _ => "Unknown".to_string(),
        };
        let mut path = percent_decode(req.uri().path().as_bytes()).decode_utf8_lossy().to_string();

        ctrl.call_next(req, depot, res).await;

        let status = res.status_code.unwrap_or_default().as_u16();
        // 是否重定向
        if let Some(p) = res.headers().get(LOCATION) {
            path.push_str(" -> ");
            path.push_str(&percent_decode(p.as_bytes()).decode_utf8_lossy());
        }

        let elapsed = started_at.elapsed();

        let msg = Message {
            begin,
            elapsed,
            status,
            ip,
            method,
            path,
            other: depot.get::<Arc<str>>("other").cloned().unwrap_or_default(),
            error: depot.get::<Arc<str>>("error").cloned().unwrap_or_default(),
        };

        if let Err(err) = self.sender.send(msg) {
            eprintln!("日志线程已停止，无法发送访问日志: {err}");
        }
    }
}
