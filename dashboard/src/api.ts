export type Miner = {
  worker: string;
  difficulty: number;
  best_difficulty: number;
  shares: number;
  rejected: number;
  stale: number;
  hashrate_gh: number;
  last_seen: string;
  last_share_time: string | null;
  notify_to_submit_ms: number;  // hashing time estimate (NOT network RTT)
  submit_rtt_ms: number;        // actual pool processing time (should be 1–5 ms)
  /** @deprecated renamed to notify_to_submit_ms */
  latency_ms_avg?: number;
  session_id: string | null;
};

export type Metrics = {
  total_hashrate_gh: number;
  total_shares: number;
  total_rejected: number;
  total_blocks: number;
  updated_at: string;
};

export type HashrateResponse = {
  total_hashrate_gh: number;
  updated_at: string;
  recent: { timestamp: string; shares: number }[];
};

export type BlockRow = {
  height: number;
  hash: string;
  status: string;
};

export type NetworkInfo = {
  blocks: number;
  difficulty: number;
  networkhashps: number;
};

const configuredBase = (import.meta.env.VITE_API_BASE ?? "").trim();
const configuredBases = (import.meta.env.VITE_API_BASES ?? "").trim();

const apiUrl = (base: string, path: string) => {
  const trimmed = base.replace(/\/$/, "");
  return `${trimmed}${path}`;
};

async function fetchJson<T>(url: string, path: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`API ${path} failed`);
  return res.json() as Promise<T>;
}

function getBases(): string[] {
  if (configuredBases)
    return configuredBases.split(",").map((s: string) => s.trim()).filter(Boolean);
  if (configuredBase) return [configuredBase];
  return ["/api"];
}

async function apiGetAll<T>(path: string): Promise<T[]> {
  const bases = getBases();
  const urls = bases.map((b) => apiUrl(b, path));
  const results = await Promise.allSettled(urls.map((u) => fetchJson<T>(u, path)));
  const ok = results
    .filter((r): r is PromiseFulfilledResult<T> => r.status === "fulfilled")
    .map((r) => r.value);
  if (ok.length > 0) return ok;
  if (!configuredBase && !configuredBases) {
    const hostname = typeof window !== "undefined" ? window.location.hostname : "localhost";
    return [await fetchJson<T>(apiUrl(`http://${hostname}:8081`, path), path)];
  }
  throw new Error(`API ${path} failed`);
}

function sumNumbers<T extends Record<string, any>>(items: T[], key: keyof T): number {
  return items.reduce((acc, it) => acc + (Number(it[key]) || 0), 0);
}
function maxString(items: { [k: string]: any }[], key: string): string {
  return items.map((it) => String(it[key] ?? "")).filter(Boolean).sort().slice(-1)[0] ?? "";
}
async function apiGetMerged<T>(path: string, merge: (parts: any[]) => T): Promise<T> {
  return merge(await apiGetAll<any>(path));
}

export const fetchMetrics = () =>
  apiGetMerged<Metrics>("/metrics", (parts) => ({
    total_hashrate_gh: sumNumbers(parts, "total_hashrate_gh"),
    total_shares: sumNumbers(parts, "total_shares"),
    total_rejected: sumNumbers(parts, "total_rejected"),
    total_blocks: sumNumbers(parts, "total_blocks"),
    updated_at: maxString(parts, "updated_at"),
  }));

export const fetchMiners = () =>
  apiGetMerged<Miner[]>("/miners", (parts) => {
    const all: Miner[] = ([] as Miner[]).concat(...parts);
    const byWorker = new Map<string, Miner>();
    for (const m of all) {
      const prev = byWorker.get(m.worker);
      if (!prev) {
        byWorker.set(m.worker, { ...m });
      } else {
        byWorker.set(m.worker, {
          ...prev,
          difficulty: Math.max(prev.difficulty, m.difficulty),
          best_difficulty: Math.max(prev.best_difficulty ?? 0, m.best_difficulty ?? 0),
          shares: prev.shares + m.shares,
          rejected: prev.rejected + m.rejected,
          stale: (prev.stale ?? 0) + (m.stale ?? 0),
          hashrate_gh: prev.hashrate_gh + m.hashrate_gh,
          last_seen: [prev.last_seen, m.last_seen].sort().slice(-1)[0],
          notify_to_submit_ms:
            prev.notify_to_submit_ms && m.notify_to_submit_ms
              ? (prev.notify_to_submit_ms + m.notify_to_submit_ms) / 2
              : prev.notify_to_submit_ms || m.notify_to_submit_ms,
          submit_rtt_ms:
            prev.submit_rtt_ms && m.submit_rtt_ms
              ? (prev.submit_rtt_ms + m.submit_rtt_ms) / 2
              : prev.submit_rtt_ms || m.submit_rtt_ms,
        });
      }
    }
    return Array.from(byWorker.values()).sort((a, b) => b.hashrate_gh - a.hashrate_gh);
  });

export const fetchHashrate = () =>
  apiGetMerged<HashrateResponse>("/hashrate", (parts) => {
    const byTs = new Map<string, number>();
    for (const p of parts as HashrateResponse[])
      for (const r of p.recent ?? [])
        byTs.set(r.timestamp, (byTs.get(r.timestamp) ?? 0) + (r.shares ?? 0));
    return {
      total_hashrate_gh: sumNumbers(parts, "total_hashrate_gh"),
      updated_at: maxString(parts, "updated_at"),
      recent: Array.from(byTs.entries())
        .map(([timestamp, shares]) => ({ timestamp, shares }))
        .sort((a, b) => (a.timestamp < b.timestamp ? -1 : 1))
        .slice(-60),
    };
  });

export const fetchBlocks = () =>
  apiGetMerged<BlockRow[]>("/blocks", (parts) => {
    const uniq = new Map<string, BlockRow>();
    for (const b of ([] as BlockRow[]).concat(...parts))
      uniq.set(`${b.height}:${b.hash}`, b);
    return Array.from(uniq.values()).sort((a, b) => b.height - a.height);
  });

export const fetchNetwork = async (): Promise<NetworkInfo | null> => {
  try {
    const bases = getBases();
    const url = apiUrl(bases[0], "/network");
    return await fetchJson<NetworkInfo>(url, "/network");
  } catch {
    return null;
  }
};
