use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Notify, Semaphore},
    task::{JoinHandle, JoinSet},
};

#[derive(Clone)]
struct Response {
    status: u16,
    body: String,
    gate: Option<Arc<Semaphore>>,
}
#[derive(Default)]
struct State {
    responses: Mutex<HashMap<String, Response>>,
    hits: Mutex<HashMap<String, usize>>,
    changed: Notify,
}

pub struct TestUpstream {
    base: String,
    state: Arc<State>,
    task: JoinHandle<()>,
}

impl TestUpstream {
    pub async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(State::default());
        let shared = state.clone();
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let Ok((mut socket, _)) = accepted else { break; };
                        let state = shared.clone();
                        connections.spawn(async move {
                            let mut request = Vec::new();
                            while request.len() < 16_384 {
                                let mut buffer = [0u8; 1024];
                                let Ok(count) = socket.read(&mut buffer).await else { return; };
                                if count == 0 { return; }
                                request.extend_from_slice(&buffer[..count]);
                                if request.windows(4).any(|window| window == b"\r\n\r\n") { break; }
                            }
                            let request = String::from_utf8_lossy(&request);
                            let path = request.split_whitespace().nth(1).unwrap_or("/").split('?').next().unwrap().to_owned();
                            let response = state.responses.lock().unwrap().get(&path).cloned().unwrap_or(Response { status: 404, body: "missing".into(), gate: None });
                            *state.hits.lock().unwrap().entry(path).or_default() += 1;
                            state.changed.notify_one();
                            if let Some(gate) = response.gate { let _permit = gate.acquire().await.unwrap(); }
                            let headers = format!("HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.status, response.body.len());
                            if socket.write_all(headers.as_bytes()).await.is_ok() { let _ = socket.write_all(response.body.as_bytes()).await; }
                        });
                    }
                    Some(_) = connections.join_next(), if !connections.is_empty() => {}
                }
            }
        });
        Self { base, state, task }
    }
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
    pub fn hits(&self, path: &str) -> usize {
        *self.state.hits.lock().unwrap().get(path).unwrap_or(&0)
    }
    pub fn set_response(&self, path: &str, status: u16, body: impl Into<String>) {
        self.state.responses.lock().unwrap().insert(
            path.into(),
            Response {
                status,
                body: body.into(),
                gate: None,
            },
        );
    }
    #[allow(dead_code)]
    pub fn block(&self, path: &str) -> Arc<Semaphore> {
        let gate = Arc::new(Semaphore::new(0));
        self.state
            .responses
            .lock()
            .unwrap()
            .get_mut(path)
            .unwrap()
            .gate = Some(gate.clone());
        gate
    }
    #[allow(dead_code)]
    pub async fn wait_for_hits(&self, path: &str, expected: usize) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while self.hits(path) < expected {
                self.state.changed.notified().await;
            }
        })
        .await
        .expect("request never reached test origin");
    }
}
impl Drop for TestUpstream {
    fn drop(&mut self) {
        self.task.abort();
    }
}
