mod api;
mod config;
mod hash;
mod metrics;
mod rpc;
mod share;
mod storage;
mod stratum;
mod template;
mod vardiff;

use std::sync::Arc;

use anyhow::Context;
use tracing::{error, info};

use crate::api::ApiServer;
use crate::config::Config;
use crate::metrics::MetricsStore;
use crate::rpc::RpcClient;
use crate::storage::{RedisStore, SqliteStore};
use crate::stratum::StratumServer;
use crate::template::TemplateEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    eprintln!("solo-pool booting");
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if let Err(err) = run().await {
        eprintln!("solo-pool fatal: {err:?}");
        return Err(err);
    }

    eprintln!("solo-pool exited cleanly");
    Ok(())
}

async fn run() -> anyhow::Result<()> {
    let config = Config::from_env().context("load config")?;
    let metrics = MetricsStore::new();

    
// Only connect storage backends when they are actually needed.
let redis = if config.persist_shares {
    RedisStore::connect(config.redis_url.as_deref()).await?
} else {
    RedisStore::connect(None).await?
};

let sqlite_needed = config.persist_shares || config.persist_blocks;
let sqlite = if sqlite_needed {
    SqliteStore::connect(config.database_url.as_deref()).await?
} else {
    SqliteStore::connect(None).await?
};


    // Restore persisted best_difficulty for all known workers so that
    // the all-time best share counter survives pool restarts.
    if sqlite.is_enabled() {
        match sqlite.load_worker_bests().await {
            Ok(bests) => {
                for (worker, best_diff) in bests {
                    metrics.set_worker_best(&worker, best_diff).await;
                }
                info!("loaded persisted best-difficulty records from SQLite");
            }
            Err(err) => {
                tracing::warn!("could not load worker bests from SQLite: {err:?}");
            }
        }
    }

    let rpc = RpcClient::new(
        config.rpc_url.clone(),
        config.rpc_user.clone(),
        config.rpc_pass.clone(),
    );

    let template_engine = Arc::new(TemplateEngine::new(config.clone(), rpc.clone(), metrics.counters.clone()));

    template_engine.start().await?;

    let stratum = StratumServer::new(
        config.clone(),
        template_engine.clone(),
        metrics.clone(),
        redis.clone(),
        sqlite.clone(),
    );

let stratum_handle = tokio::spawn(async move { stratum.run().await });

// API server is optional (disable for max-perf endpoints).
let api_handle = if config.api_enabled {
    let api = ApiServer::new(config.clone(), metrics.clone(), sqlite, rpc, template_engine.clone());
    Some(tokio::spawn(async move { api.run().await }))
} else {
    None
};


if let Some(api_handle) = api_handle {
    tokio::select! {
        res = stratum_handle => {
            match res {
                Ok(Ok(())) => info!("stratum stopped"),
                Ok(Err(err)) => {
                    error!("stratum exited with error: {err:?}");
                    return Err(err).context("stratum exited");
                }
                Err(err) => {
                    error!("stratum task failed: {err:?}");
                    return Err(err).context("stratum task join failed");
                }
            }
        }
        res = api_handle => {
            match res {
                Ok(Ok(())) => info!("api stopped"),
                Ok(Err(err)) => {
                    error!("api exited with error: {err:?}");
                    return Err(err).context("api exited");
                }
                Err(err) => {
                    error!("api task failed: {err:?}");
                    return Err(err).context("api task join failed");
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown signal received");
        }
    }
} else {
    // API disabled: wait on stratum only (or ctrl-c)
    tokio::select! {
        res = stratum_handle => {
            match res {
                Ok(Ok(())) => info!("stratum stopped"),
                Ok(Err(err)) => {
                    error!("stratum exited with error: {err:?}");
                    return Err(err).context("stratum exited");
                }
                Err(err) => {
                    error!("stratum task failed: {err:?}");
                    return Err(err).context("stratum task join failed");
                }
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("shutdown signal received");
        }
    }
}

Ok(())
}
