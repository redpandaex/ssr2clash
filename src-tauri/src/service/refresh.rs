use super::{active, validate_source, AppService, ServedYaml};
use crate::model::app::{AppError, GeneratedConfig, OutputMode};

fn cancelled() -> AppError {
    AppError::new(
        "GenerationCancelled",
        "配置或服务状态已变化，本次更新已取消",
    )
}

impl AppService {
    pub async fn refresh(&self) -> Result<GeneratedConfig, AppError> {
        let service = self.clone();
        tokio::spawn(async move { service.refresh_owned().await })
            .await
            .map_err(|_| AppError::new("InternalTask", "转换任务异常终止"))?
    }

    async fn refresh_owned(&self) -> Result<GeneratedConfig, AppError> {
        let (profile, revision, proxy, epoch, request_id, mut events) = {
            let mut runtime = self.inner.runtime.lock().await;
            if runtime.saved.mode == OutputMode::Snapshot {
                return Err(AppError::new(
                    "SnapshotFrozen",
                    "快照已冻结，请先切回实时模式",
                ));
            }
            if runtime.in_flight >= 2 {
                return Err(AppError::new(
                    "TooManyRequests",
                    "已有两个更新正在执行，请稍后重试",
                ));
            }
            validate_source(&runtime.saved.settings, true)?;
            runtime.next_request = runtime.next_request.wrapping_add(1);
            runtime.in_flight += 1;
            runtime.log("info", "开始更新订阅");
            self.publish(&runtime);
            (
                runtime.saved.settings.profile.clone(),
                runtime.saved.revision,
                runtime.saved.settings.fetch_proxy,
                runtime.epoch,
                runtime.next_request,
                self.subscribe(),
            )
        };
        let future = self.inner.converter.generate(profile, revision, proxy);
        tokio::pin!(future);
        let result = loop {
            if *events.borrow() != epoch {
                break Err(cancelled());
            }
            tokio::select! { biased;
                changed = events.changed() => { if changed.is_err() || *events.borrow() != epoch { break Err(cancelled()); } }
                result = &mut future => break result,
            }
        };
        let result = match result {
            Ok(result) => self
                .transact(move |runtime| {
                    if runtime.epoch != epoch
                        || runtime.saved.revision != revision
                        || runtime.saved.mode != OutputMode::Live
                    {
                        return Err(cancelled());
                    }
                    if request_id < runtime.last_committed_request {
                        return Err(AppError::new(
                            "SupersededGeneration",
                            "较旧的更新结果已忽略",
                        ));
                    }
                    runtime.last_committed_request = request_id;
                    runtime.log(
                        "info",
                        format!(
                            "更新成功：{} 个节点，{} 个策略组",
                            result.output.node_count, result.output.group_count
                        ),
                    );
                    runtime.saved.last_good = Some(result.output);
                    runtime.details = Some(result.details);
                    runtime.last_error = None;
                    runtime.served_stale = false;
                    Ok(true)
                })
                .await
                .and_then(|status| {
                    status
                        .active_result
                        .ok_or_else(|| AppError::new("InternalState", "更新结果不可用"))
                }),
            Err(error) => Err(error),
        };
        let result = match result {
            Err(error) if error.code == "SupersededGeneration" => {
                let runtime = self.inner.runtime.lock().await;
                if runtime.epoch == epoch && runtime.saved.mode == OutputMode::Live {
                    active(&runtime).cloned().ok_or_else(cancelled)
                } else {
                    Err(cancelled())
                }
            }
            Err(error) if error.code != "GenerationCancelled" => {
                let mut runtime = self.inner.runtime.lock().await;
                if runtime.epoch != epoch || runtime.saved.mode != OutputMode::Live {
                    Err(cancelled())
                } else {
                    runtime.last_error = Some(error.message.clone());
                    runtime.log("error", format!("更新失败（{}）", error.code));
                    self.publish(&runtime);
                    Err(error)
                }
            }
            result => result,
        };
        let mut runtime = self.inner.runtime.lock().await;
        runtime.in_flight = runtime.in_flight.saturating_sub(1);
        self.publish(&runtime);
        result
    }

    pub async fn serve_subscription(&self) -> Result<ServedYaml, AppError> {
        {
            let mut runtime = self.inner.runtime.lock().await;
            runtime.last_request_at = Some(crate::conversion::now_rfc3339());
            if runtime.saved.mode == OutputMode::Snapshot {
                let yaml = active(&runtime)
                    .ok_or_else(|| AppError::new("NoSnapshotAvailable", "没有可用的冻结快照"))?
                    .yaml
                    .clone();
                runtime.log("info", "已返回冻结快照");
                self.publish(&runtime);
                return Ok(ServedYaml { yaml, stale: false });
            }
            self.publish(&runtime);
        }
        match self.refresh().await {
            Ok(output) => Ok(ServedYaml {
                yaml: output.yaml,
                stale: false,
            }),
            Err(error) => {
                let mut runtime = self.inner.runtime.lock().await;
                if runtime.saved.mode == OutputMode::Snapshot {
                    if let Some(output) = active(&runtime) {
                        return Ok(ServedYaml {
                            yaml: output.yaml.clone(),
                            stale: false,
                        });
                    }
                }
                if matches!(
                    error.code.as_str(),
                    "GenerationCancelled" | "TooManyRequests"
                ) {
                    return Err(error);
                }
                if let Some(output) = active(&runtime) {
                    let yaml = output.yaml.clone();
                    runtime.served_stale = true;
                    runtime.log("warning", "本次更新失败，已返回最近成功的配置");
                    self.publish(&runtime);
                    Ok(ServedYaml { yaml, stale: true })
                } else {
                    Err(error)
                }
            }
        }
    }
}
