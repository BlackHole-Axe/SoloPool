use redis::AsyncCommands;
use std::sync::Arc;

#[derive(Clone)]
pub struct RedisStore {
    client: Option<Arc<redis::Client>>,
}

impl RedisStore {
    pub async fn connect(redis_url: Option<&str>) -> anyhow::Result<Self> {
        if let Some(url) = redis_url {
            let client = redis::Client::open(url)?;
            Ok(Self {
                client: Some(Arc::new(client)),
            })
        } else {
            Ok(Self { client: None })
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    pub async fn incr_share(&self, worker: &str) -> anyhow::Result<()> {
        let Some(client) = &self.client else {
            return Ok(());
        };
        let mut conn = client.get_multiplexed_async_connection().await?;
        let key = format!("miner:{worker}:shares");
        let _: i64 = conn.incr(key, 1).await?;
        Ok(())
    }

    pub async fn set_difficulty(&self, worker: &str, diff: f64) -> anyhow::Result<()> {
        let Some(client) = &self.client else {
            return Ok(());
        };
        let mut conn = client.get_multiplexed_async_connection().await?;
        let key = format!("miner:{worker}:difficulty");
        let _: () = conn.set(key, diff).await?;
        Ok(())
    }
}
