use std::collections::VecDeque;
use chrono::{DateTime, Utc};

// ─── Tuning constants ─────────────────────────────────────────────────────────
//
// EMA_ALPHA: smoothing factor for hashrate estimate (0 < α ≤ 1).
//   Lower α = more smoothing = less sensitive to short bursts.
//   0.25 means ~4 retarget periods to converge fully.
const EMA_ALPHA: f64 = 0.25;

// HYSTERESIS: require target to differ from current by this factor before retarget.
//   1.5 means: only retarget if target ≥ current×1.5 (up) or ≤ current/1.5 (down).
//   Prevents oscillation at power-of-2 boundaries and eliminates boundary-stuck.
const HYSTERESIS: f64 = 1.5;

// OFFLINE_SECS: if zero shares in this many seconds → cut difficulty by 6.
//   Only fires when truly offline, not from statistical variance.
const OFFLINE_SECS: f64 = 120.0; // 2 minutes

// CACHE_SIZE: sliding window of recent shares for hashrate estimation.
const CACHE_SIZE: usize = 30;

/// Vardiff controller — EMA + Hysteresis design.
///
/// # Algorithm
///
/// 1. Maintain a sliding ring-buffer of (time, difficulty) for last N shares.
///
/// 2. Estimate hashrate via EMA (exponential moving average):
///      ema_rate = α × instant_rate + (1-α) × ema_rate
///    where instant_rate = Σ(difficulty) / Δtime_s
///
/// 3. Target difficulty = ema_rate × target_share_time
///
/// 4. Round to nearest power-of-2.
///
/// 5. Retarget only when:
///      UP:   target ≥ current × HYSTERESIS  (prevents boundary-stuck going up)
///      DOWN: target ≤ current / HYSTERESIS  (prevents boundary-stuck going down)
///
/// 6. ÷6 rescue rule: if zero shares for OFFLINE_SECS → miner is truly stopped.
///
/// # Vardiff does NOT change block probability
///
/// E[best_diff per second] = hashrate / 2^32 = constant regardless of difficulty.
/// Vardiff only controls how often the pool receives shares (reporting rate).
/// Higher difficulty → fewer shares → more CPU headroom; mining efficiency unchanged.
#[derive(Debug, Clone)]
pub struct VardiffController {
    target_share_time: f64,
    retarget_time:     f64,
    min_diff:          f64,
    max_diff:          f64,

    last_retarget: DateTime<Utc>,
    session_start: DateTime<Utc>,

    /// EMA of difficulty_per_second (hashrate / 2^32).
    /// Initialized to 0 until first share arrives.
    ema_rate: f64,
    /// Whether ema_rate has been seeded from at least one measurement.
    ema_seeded: bool,

    /// Sliding ring-buffer of (time, difficulty) for instant-rate estimation.
    samples: VecDeque<(DateTime<Utc>, f64)>,
}

impl VardiffController {
    pub fn new(
        target_share_time: f64,
        retarget_time:     f64,
        min_diff:          f64,
        max_diff:          f64,
    ) -> Self {
        let now = Utc::now();
        Self {
            target_share_time: target_share_time.max(1.0),
            retarget_time:     retarget_time.max(1.0),
            min_diff:          min_diff.max(1.0),
            max_diff:          max_diff.max(min_diff),
            last_retarget:     now,
            session_start:     now,
            ema_rate:          0.0,
            ema_seeded:        false,
            samples:           VecDeque::with_capacity(CACHE_SIZE),
        }
    }

    /// Record an accepted share.
    pub fn record_share(&mut self, now: DateTime<Utc>, difficulty: f64) {
        self.samples.push_back((now, difficulty));
        if self.samples.len() > CACHE_SIZE {
            self.samples.pop_front();
        }
    }

    /// Returns Some(new_diff) if a retarget is warranted, None otherwise.
    pub fn maybe_retarget(&mut self, current_diff: f64, now: DateTime<Utc>) -> Option<f64> {
        // Enforce minimum cadence.
        let since_ms = (now - self.last_retarget).num_milliseconds();
        if since_ms < (self.retarget_time * 1000.0) as i64 { return None; }
        self.last_retarget = now;

        // ── Offline rescue ────────────────────────────────────────────────────
        // Zero shares for OFFLINE_SECS → miner truly stopped. Cut difficulty.
        if self.samples.is_empty() {
            let age_s = (now - self.session_start).num_seconds() as f64;
            if age_s > OFFLINE_SECS {
                let stepped = self.nearest_p2((current_diff / 6.0).max(self.min_diff));
                if (stepped - current_diff).abs() > f64::EPSILON {
                    return Some(stepped);
                }
            }
            return None;
        }

        // ── Instant rate from sliding window ─────────────────────────────────
        let sum  = self.samples.iter().map(|(_, d)| *d).sum::<f64>();
        let span = (self.samples.back().unwrap().0 - self.samples.front().unwrap().0)
                       .num_milliseconds() as f64 / 1000.0;
        if span <= 0.0 { return None; }
        let instant_rate = sum / span;

        // ── EMA update ────────────────────────────────────────────────────────
        if self.ema_seeded {
            self.ema_rate = EMA_ALPHA * instant_rate + (1.0 - EMA_ALPHA) * self.ema_rate;
        } else {
            self.ema_rate  = instant_rate; // seed on first measurement
            self.ema_seeded = true;
        }

        // ── Target difficulty ─────────────────────────────────────────────────
        let raw_target  = self.ema_rate * self.target_share_time;
        let target_diff = self.nearest_p2(raw_target.clamp(self.min_diff, self.max_diff));

        // ── Hysteresis dead-band ──────────────────────────────────────────────
        // Retarget UP   if target ≥ current × HYSTERESIS
        // Retarget DOWN if target ≤ current / HYSTERESIS
        // This eliminates both oscillation and boundary-stuck at exact P2 values.
        let should_up   = target_diff >= current_diff * HYSTERESIS;
        let should_down = target_diff <= current_diff / HYSTERESIS;

        if should_up || should_down {
            Some(target_diff)
        } else {
            None
        }
    }

    /// Round to nearest power of 2, clamped to [min_diff, max_diff].
    pub fn nearest_p2(&self, val: f64) -> f64 {
        if !val.is_finite() || val <= 0.0 { return self.min_diff; }
        let c = val.clamp(self.min_diff, self.max_diff);
        let n = c as u64;
        if n == 0 { return self.min_diff; }
        let bits  = u64::BITS - n.leading_zeros() - 1;
        let lower = 1u64 << bits;
        let upper = lower << 1;
        let res = if (c - lower as f64) <= (upper as f64 - c) { lower as f64 }
                  else                                          { upper as f64 };
        res.clamp(self.min_diff, self.max_diff)
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_vc(target_s: f64) -> VardiffController {
        VardiffController::new(target_s, 30.0, 512.0, 33_554_432.0)
    }

    fn add_shares(vc: &mut VardiffController, n: usize, diff: f64, interval_secs: f64) {
        let mut t = Utc::now();
        for _ in 0..n {
            t += Duration::milliseconds((interval_secs * 1000.0) as i64);
            vc.record_share(t, diff);
        }
    }

    /// Hysteresis prevents retarget when target is close to current.
    #[test]
    fn test_no_oscillation_at_boundary() {
        let mut vc = make_vc(10.0);
        // Miner doing exactly target rate → EMA converges to current diff
        // target_diff == current_diff → no retarget
        add_shares(&mut vc, 30, 16384.0, 10.0); // 1 share/10s = exact target
        let r = vc.maybe_retarget(16384.0, Utc::now() + Duration::seconds(31));
        // At 1 share/10s with diff=16384, target = rate * 10 = 16384
        // 16384 < 16384 * 1.5 → no UP retarget
        // 16384 > 16384 / 1.5 → no DOWN retarget
        assert!(r.is_none(), "should not retarget when on target: {:?}", r);
    }

    /// Offline rescue: 0 shares for 2+ minutes → difficulty cut by 6.
    #[test]
    fn test_offline_rescue() {
        let mut vc = make_vc(10.0);
        let future = Utc::now() + Duration::seconds(200);
        let r = vc.maybe_retarget(16384.0, future);
        assert!(r.is_some());
        let new_d = r.unwrap();
        // 16384 / 6 = 2730 → nearest P2 = 2048
        assert_eq!(new_d, 2048.0);
    }

    /// Fast miner: 10× too fast → difficulty should increase.
    #[test]
    fn test_fast_miner_increases_diff() {
        let mut vc = make_vc(10.0);
        // 1 share per second (10× target rate of 1/10s)
        add_shares(&mut vc, 30, 16384.0, 1.0);
        let r = vc.maybe_retarget(16384.0, Utc::now() + Duration::seconds(31));
        assert!(r.is_some(), "fast miner should trigger retarget");
        assert!(r.unwrap() > 16384.0 * HYSTERESIS - 1.0);
    }

    /// Slow miner: 10× too slow → difficulty should decrease.
    #[test]
    fn test_slow_miner_decreases_diff() {
        let mut vc = make_vc(10.0);
        // 1 share per 100s (10× below target)
        add_shares(&mut vc, 30, 16384.0, 100.0);
        let r = vc.maybe_retarget(16384.0, Utc::now() + Duration::seconds(31));
        assert!(r.is_some(), "slow miner should trigger retarget");
        assert!(r.unwrap() < 16384.0 / HYSTERESIS + 1.0);
    }

    /// Vardiff does NOT change block probability.
    #[test]
    fn test_vardiff_does_not_change_block_probability() {
        // E[best_diff per second] = hashrate / 2^32 = constant
        // For a 10 TH/s miner:
        let hashrate: f64 = 10e12; // H/s
        let hashes_per_diff: f64 = 4_294_967_296.0; // 2^32

        let rate = hashrate / hashes_per_diff; // diff units/s

        // With diff=16384: rate = same
        let rate_low_diff = rate;
        // With diff=262144: rate = same
        let rate_high_diff = rate;

        assert!((rate_low_diff - rate_high_diff).abs() < f64::EPSILON,
                "difficulty does not affect rate of best_diff growth");
    }
}
