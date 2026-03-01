import React, { useEffect, useState } from "react";
import {
  fetchBlocks, fetchHashrate, fetchMetrics, fetchMiners, fetchNetwork,
  BlockRow, HashrateResponse, Metrics, Miner, NetworkInfo
} from "./api";
import StatsCard from "./components/StatsCard";
import HashrateChart from "./components/HashrateChart";
import MinersTable from "./components/MinersTable";
import BlocksTable from "./components/BlocksTable";

const REFRESH_MS = 5000;

/** Universal hashrate formatter — KH/s → MH/s → GH/s → TH/s → PH/s → EH/s → ZH/s */
function fmtHr(gh: number): string {
  const hs = gh * 1e9;
  if (hs >= 1e21) return `${(hs / 1e21).toFixed(3)} ZH/s`;
  if (hs >= 1e18) return `${(hs / 1e18).toFixed(3)} EH/s`;
  if (hs >= 1e15) return `${(hs / 1e15).toFixed(3)} PH/s`;
  if (hs >= 1e12) return `${(hs / 1e12).toFixed(2)} TH/s`;
  if (hs >= 1e9)  return `${(hs / 1e9).toFixed(2)} GH/s`;
  if (hs >= 1e6)  return `${(hs / 1e6).toFixed(2)} MH/s`;
  if (hs >= 1e3)  return `${(hs / 1e3).toFixed(2)} KH/s`;
  return `${hs.toFixed(0)} H/s`;
}

/** K → M → G → T → P → E  (same SI scale as hashrate) */
function fmtBest(d: number): string {
  if (d <= 0)    return "—";
  if (d >= 1e18) return `${(d / 1e18).toFixed(3)} E`;
  if (d >= 1e15) return `${(d / 1e15).toFixed(3)} P`;
  if (d >= 1e12) return `${(d / 1e12).toFixed(3)} T`;
  if (d >= 1e9)  return `${(d / 1e9).toFixed(3)} G`;
  if (d >= 1e6)  return `${(d / 1e6).toFixed(2)} M`;
  if (d >= 1e3)  return `${(d / 1e3).toFixed(1)} K`;
  return d.toFixed(0);
}

function fmtDiff(d: number): string {
  if (d >= 1e15) return `${(d / 1e15).toFixed(3)} P`;
  if (d >= 1e12) return `${(d / 1e12).toFixed(2)} T`;
  if (d >= 1e9)  return `${(d / 1e9).toFixed(2)} G`;
  if (d >= 1e6)  return `${(d / 1e6).toFixed(2)} M`;
  if (d >= 1e3)  return `${(d / 1e3).toFixed(1)} K`;
  return d.toFixed(0);
}

/** Same universal formatter for network hashrate (input in H/s) */
function fmtNetHash(h: number): string {
  if (h >= 1e21) return `${(h / 1e21).toFixed(3)} ZH/s`;
  if (h >= 1e18) return `${(h / 1e18).toFixed(3)} EH/s`;
  if (h >= 1e15) return `${(h / 1e15).toFixed(3)} PH/s`;
  if (h >= 1e12) return `${(h / 1e12).toFixed(2)} TH/s`;
  if (h >= 1e9)  return `${(h / 1e9).toFixed(2)} GH/s`;
  if (h >= 1e6)  return `${(h / 1e6).toFixed(2)} MH/s`;
  return `${(h / 1e3).toFixed(2)} KH/s`;
}

export default function App() {
  const [metrics, setMetrics] = useState<Metrics | null>(null);
  const [miners, setMiners] = useState<Miner[]>([]);
  const [hashrate, setHashrate] = useState<HashrateResponse | null>(null);
  const [blocks, setBlocks] = useState<BlockRow[]>([]);
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [live, setLive] = useState(true);

  const loadAll = async () => {
    try {
      const [m, mn, h, b, n] = await Promise.all([
        fetchMetrics(), fetchMiners(), fetchHashrate(), fetchBlocks(), fetchNetwork()
      ]);
      setMetrics(m); setMiners(mn); setHashrate(h); setBlocks(b); setNetwork(n);
      setLive(true);
    } catch {
      setLive(false);
    }
  };

  useEffect(() => {
    loadAll();
    const t = setInterval(loadAll, REFRESH_MS);
    return () => clearInterval(t);
  }, []);

  // derive best_difficulty across all miners
  const bestDiff = miners.reduce((mx, m) => Math.max(mx, m.best_difficulty ?? 0), 0);
  const totalStale = miners.reduce((s, m) => s + (m.stale ?? 0), 0);

  return (
    <div className="app">

      {/* ── Header ── */}
      <header className="header">
        <div className="header-left">
          <div>
            <div className="header-logo">⬡ SOLO POOL</div>
            <div className="header-sub">Solo Bitcoin Mining — Real-time Dashboard</div>
          </div>
        </div>
        <div className="header-right">
          {network && (
            <div className="badge" style={{ background: "rgba(168,85,247,.1)", border: "1px solid rgba(168,85,247,.25)", color: "var(--purple)", fontSize: 12, fontFamily: "JetBrains Mono, monospace" }}>
              ⛓ {network.blocks.toLocaleString()}
            </div>
          )}
          <div className={`badge ${live ? "badge-live" : "badge-offline"}`}>
            {live ? "LIVE" : "OFFLINE"}
          </div>
        </div>
      </header>

      {/* ── Network Info Bar ── */}
      {network && (
        <div className="net-bar">
          <div className="net-item">
            <span className="net-label">Height</span>
            <span className="net-value">{network.blocks.toLocaleString()}</span>
          </div>
          <span className="net-sep">·</span>
          <div className="net-item">
            <span className="net-label">Network Diff</span>
            <span className="net-value">{fmtDiff(network.difficulty)}</span>
          </div>
          <span className="net-sep">·</span>
          <div className="net-item">
            <span className="net-label">Network Hashrate</span>
            <span className="net-value">{fmtNetHash(network.networkhashps ?? 0)}</span>
          </div>
          <span className="net-sep">·</span>
          <div className="net-item">
            <span className="net-label">Pool Share</span>
            <span className="net-value">
              {network.networkhashps > 0
                ? `${((metrics?.total_hashrate_gh ?? 0) * 1e9 / network.networkhashps * 100).toFixed(7)}%`
                : "—"}
            </span>
          </div>
        </div>
      )}

      {/* ── Stats Cards ── */}
      <div className="stats-grid">
        <StatsCard
          icon="⚡"
          label="Pool Hashrate"
          value={fmtHr(metrics?.total_hashrate_gh ?? 0)}
          sub={`${miners.length} miners active`}
          accent="cyan"
          valueColor="cyan"
        />
        <StatsCard
          icon="★"
          label="Best Share"
          value={fmtBest(bestDiff)}
          sub={bestDiff > 0 ? `${(bestDiff / (network?.difficulty ?? 144e12) * 100).toFixed(6)}% of network` : "No share yet"}
          accent="yellow"
          valueColor="yellow"
        />
        <StatsCard
          icon="✓"
          label="Accepted Shares"
          value={(metrics?.total_shares ?? 0).toLocaleString()}
          sub={`Rejected: ${metrics?.total_rejected ?? 0}  ·  Stale: ${totalStale}`}
          accent="green"
          valueColor="green"
        />
        <StatsCard
          icon="◈"
          label="Active Miners"
          value={`${miners.length}`}
          sub="Workers connected"
          accent="purple"
          valueColor="purple"
        />
        <StatsCard
          icon="⬡"
          label="Blocks Found"
          value={`${metrics?.total_blocks ?? 0}`}
          sub="Submitted to network"
          accent="orange"
          valueColor={metrics?.total_blocks ? "orange" : "text"}
        />
        <StatsCard
          icon="◷"
          label="Last Update"
          value={metrics?.updated_at ? new Date(metrics.updated_at).toLocaleTimeString() : "—"}
          sub="Refresh every 5s"
          accent="cyan"
          valueColor="text"
        />
      </div>

      {/* ── Chart + Blocks ── */}
      <div className="main-grid section-gap">
        <HashrateChart data={hashrate?.recent ?? []} />
        <BlocksTable blocks={blocks} />
      </div>

      {/* ── Miners Table ── */}
      <MinersTable miners={miners} />

    </div>
  );
}
