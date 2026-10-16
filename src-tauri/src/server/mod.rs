mod handlers;

use crate::{model::app::AppError, service::AppService};
use std::{
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex},
    task::JoinHandle,
};

#[derive(Clone)]
pub struct ServerController {
    service: Arc<AppService>,
    running: Arc<Mutex<Option<Running>>>,
}

struct Running {
    accepting: Arc<AtomicBool>,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<std::io::Result<()>>,
}

impl ServerController {
    pub fn new(service: Arc<AppService>) -> Self {
        Self {
            service,
            running: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self, port: u16) -> Result<(), AppError> {
        let controller = self.clone();
        tokio::spawn(async move { controller.start_owned(port).await })
            .await
            .map_err(|_| AppError::new("InternalTask", "启动服务任务未能完成"))?
    }

    async fn start_owned(&self, port: u16) -> Result<(), AppError> {
        let mut current = self.running.lock().await;
        if current
            .as_ref()
            .is_some_and(|running| !running.task.is_finished())
        {
            return Ok(());
        }
        *current = None;
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))
            .await
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AddrInUse {
                    AppError::new("PortInUse", "本地端口已被占用，请停止占用程序或修改端口")
                } else {
                    AppError::new("ServerBind", "无法监听本地端口，请检查端口和系统权限")
                }
            })?;
        let actual_port = listener
            .local_addr()
            .map_err(|_| AppError::new("ServerBind", "无法读取监听端口"))?
            .port();
        // Binding and intent persistence both succeed before the listener is exposed as running.
        self.service.set_service_enabled(true).await?;
        let accepting = Arc::new(AtomicBool::new(true));
        let app = handlers::router(self.service.clone(), accepting.clone());
        let (shutdown, signal) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = signal.await;
                })
                .await
        });
        *current = Some(Running {
            accepting,
            shutdown,
            task,
        });
        self.service.mark_server_running(Some(actual_port)).await;
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), AppError> {
        let controller = self.clone();
        tokio::spawn(async move { controller.stop_owned().await })
            .await
            .map_err(|_| AppError::new("InternalTask", "停止服务任务未能完成"))?
    }

    async fn stop_owned(&self) -> Result<(), AppError> {
        let mut current = self.running.lock().await;
        if let Some(mut running) = current.take() {
            running.accepting.store(false, Ordering::Release);
            let _ = running.shutdown.send(());
            self.service.cancel_refreshes().await;
            if tokio::time::timeout(Duration::from_secs(2), &mut running.task)
                .await
                .is_err()
            {
                running.task.abort();
                let _ = running.task.await;
            }
            self.service.mark_server_running(None).await;
        } else {
            self.service.cancel_refreshes().await;
        }
        // A failed disk write must still leave the actual listener stopped.
        self.service.set_service_enabled(false).await?;
        Ok(())
    }
}
