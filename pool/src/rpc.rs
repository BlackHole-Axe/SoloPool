use anyhow::{anyhow, Context};
use reqwest::Client;
use std::time::Duration;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct RpcClient {
    url:            String,
    user:           String,
    pass:           String,
    /// Fast client: 8s timeout for all normal RPC calls (submit, getinfo, etc.)
    client:         Client,
    /// Slow client: 130s timeout exclusively for getblocktemplate with longpollid.
    /// Bitcoin Core holds the connection open for ~90s until the template changes.
    /// Using the fast client for longpoll causes "RPC request failed" every 8s.
    client_longpoll: Client,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcClient {
    pub fn new(url: String, user: String, pass: String) -> Self {
        // Fast client: tight timeouts to fail quickly on network issues.
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(8))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(8)
            .build()
            .expect("build reqwest client");

        // Longpoll client: 130s response timeout (Core holds ~90s, +40s buffer).
        // connect_timeout stays short so a dead Core is detected quickly.
        let client_longpoll = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(130))
            .tcp_nodelay(true)
            .pool_max_idle_per_host(2)
            .build()
            .expect("build longpoll reqwest client");

        Self { url, user, pass, client, client_longpoll }
    }

    /// Normal RPC call — 8s timeout. Use for everything except longpoll GBT.
    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<T> {
        self.call_with_client(&self.client, method, params).await
    }

    /// Longpoll-aware GBT call — 130s timeout.
    /// Blocks until Bitcoin Core returns a changed template (up to ~90s).
    /// Eliminates the "longpoll GBT call failed" WARN spam.
    pub async fn call_longpoll<T: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<T> {
        self.call_with_client(&self.client_longpoll, method, params).await
    }

    async fn call_with_client<T: DeserializeOwned>(
        &self,
        client: &Client,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<T> {
        let payload = json!({
            "jsonrpc": "1.0",
            "id": "Solo",
            "method": method,
            "params": params,
        });

        let response = client
            .post(&self.url)
            .basic_auth(&self.user, Some(&self.pass))
            .json(&payload)
            .send()
            .await
            .context("RPC request failed")?;

        let status = response.status();
        let body = response.json::<RpcResponse<T>>().await?;

        if let Some(err) = body.error {
            return Err(anyhow!("RPC error {method}: {} ({})", err.message, err.code));
        }

        body.result.ok_or_else(|| anyhow!("RPC {method} returned empty result (status {status})"))
    }
}
