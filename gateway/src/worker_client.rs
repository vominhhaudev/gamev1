use std::time::Duration;
use tonic::transport::{Channel, Endpoint};
use tracing::info;

// Nếu máy bạn không có module v1, đổi thành: use proto::worker::{ ... };
use proto::worker::v1::{worker_client::WorkerClient as PbClient, PushInputRequest};

#[derive(Clone)]
pub struct WorkerClient {
    inner: PbClient<Channel>,
}

impl WorkerClient {
    pub async fn connect(uri: &str) -> anyhow::Result<Self> {
        let ep = Endpoint::from_shared(uri.to_string())?
            .connect_timeout(Duration::from_secs(2))
            .tcp_keepalive(Some(Duration::from_secs(10)));
        match ep.connect().await {
            Ok(channel) => {
                info!(%uri, "gateway connected to worker");
                Ok(Self {
                    inner: PbClient::new(channel),
                })
            }
            Err(e) => {
                // Không chặn khởi động gateway nếu worker chưa chạy: dùng connect_lazy.
                tracing::warn!(error=?e, %uri, "worker not available; using lazy channel");
                let channel = Endpoint::from_shared(uri.to_string())?.connect_lazy();
                Ok(Self {
                    inner: PbClient::new(channel),
                })
            }
        }
    }

    pub async fn push_input(&self, req: PushInputRequest) -> anyhow::Result<()> {
        let mut c = self.inner.clone();
        c.push_input(req).await?;
        Ok(())
    }
}
