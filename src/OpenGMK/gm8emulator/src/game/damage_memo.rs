//! Reactive Memoization of Combat Damage Formulas and Stat Stage Multipliers.
//!
//! Avoids recomputing `attack * stages * type_bonus * weather` repeatedly inside
//! high-frequency collision/projectile loops. Values are invalidated via dirty flags
//! only when a Pokémon's stat stage, party slot, or status condition changes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static DAMAGE_MEMO_ENABLED: AtomicBool = AtomicBool::new(true);

#[inline(always)]
pub fn is_damage_memo_enabled() -> bool {
    DAMAGE_MEMO_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_damage_memo_enabled(enabled: bool) {
    DAMAGE_MEMO_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Cache key: (Attacker Object ID, Defender Object ID, Move ID, Weather State)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DamageKey {
    pub attacker_id: i32,
    pub defender_id: i32,
    pub move_id: i32,
    pub weather: u8,
}

#[derive(Default)]
pub struct DamageMemoCache {
    entries: HashMap<DamageKey, f32>,
    dirty: bool,
}

static GLOBAL_DAMAGE_CACHE: std::sync::LazyLock<Mutex<DamageMemoCache>> =
    std::sync::LazyLock::new(|| Mutex::new(DamageMemoCache::default()));

/// Invalidates the damage memoization cache when game state shifts
/// (e.g. Pokémon switches, stat boost/drop, status condition changes).
pub fn invalidate_damage_cache() {
    if let Ok(mut cache) = GLOBAL_DAMAGE_CACHE.lock() {
        cache.entries.clear();
        cache.dirty = false;
    }
}

/// Queries the damage memo cache. Returns Some(multiplier) if cached, or None if computation is needed.
#[inline]
pub fn query_damage_memo(key: DamageKey) -> Option<f32> {
    if !is_damage_memo_enabled() {
        return None;
    }
    if let Ok(cache) = GLOBAL_DAMAGE_CACHE.lock() {
        cache.entries.get(&key).copied()
    } else {
        None
    }
}

/// Inserts a computed damage multiplier into the memo cache.
#[inline]
pub fn store_damage_memo(key: DamageKey, multiplier: f32) {
    if !is_damage_memo_enabled() {
        return;
    }
    if let Ok(mut cache) = GLOBAL_DAMAGE_CACHE.lock() {
        // Limit cache size to avoid unbounded memory growth in long sessions
        if cache.entries.len() > 2048 {
            cache.entries.clear();
        }
        cache.entries.insert(key, multiplier);
    }
}

/// Precomputed stat stage multiplier table (-6 to +6).
/// Stages 0..=6: (2 + stage) / 2
/// Stages -6..0: 2 / (2 - stage)
const STAT_STAGES: [f32; 13] = [
    2.0 / 8.0, // -6
    2.0 / 7.0, // -5
    2.0 / 6.0, // -4
    2.0 / 5.0, // -3
    2.0 / 4.0, // -2
    2.0 / 3.0, // -1
    1.0,       //  0
    3.0 / 2.0, // +1
    4.0 / 2.0, // +2
    5.0 / 2.0, // +3
    6.0 / 2.0, // +4
    7.0 / 2.0, // +5
    8.0 / 2.0, // +6
];

#[inline(always)]
pub fn fast_stat_stage_multiplier(stage: i8) -> f32 {
    let clamped = stage.clamp(-6, 6) + 6;
    STAT_STAGES[clamped as usize]
}
