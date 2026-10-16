use super::AppService;
use crate::model::app::{AppError, CoreStatus, OutputMode};

impl AppService {
    pub async fn set_snapshot(&self, enabled: bool) -> Result<CoreStatus, AppError> {
        let desired = if enabled {
            OutputMode::Snapshot
        } else {
            OutputMode::Live
        };
        self.transact(move |runtime| {
            if runtime.saved.mode == desired {
                return Ok(false);
            }
            if enabled {
                let snapshot = runtime
                    .saved
                    .last_good
                    .as_ref()
                    .filter(|output| output.config_revision == runtime.saved.revision)
                    .ok_or_else(|| {
                        AppError::new("NoSnapshotAvailable", "请先成功生成一次配置，再开启快照")
                    })?
                    .clone();
                runtime.saved.frozen = Some(snapshot);
            }
            runtime.saved.mode = desired;
            runtime.epoch = runtime.epoch.wrapping_add(1);
            runtime.last_error = None;
            runtime.served_stale = false;
            runtime.log(
                "info",
                if enabled {
                    "已冻结当前配置，订阅请求将直接返回快照"
                } else {
                    "已恢复实时模式，下次订阅请求将重新拉取"
                },
            );
            Ok(true)
        })
        .await
    }
}
