//! AOT & Native Fast-Path Execution Engine for GML Heavy Compute Loops.
//!
//! Provides precompiled native Rust/ARM64 implementations for math-intensive formulas
//! (battle damage calculations, distance checks, angle wrapping, and depth sorting),
//! bypassing bytecode interpretation overhead with zero race conditions.

use std::collections::HashMap;
use std::sync::RwLock;
use crate::gml::{self, Context, Value};
use crate::math::Real;

pub type NativeScriptFn = fn(&mut crate::game::Game, &mut Context, &[Value]) -> gml::Result<Value>;

pub struct GmlJitEngine {
    enabled: bool,
    hooks: RwLock<HashMap<usize, NativeScriptFn>>,
}

impl GmlJitEngine {
    pub fn new() -> Self {
        let engine = Self {
            enabled: true,
            hooks: RwLock::new(HashMap::new()),
        };
        engine
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn register_hook(&self, script_id: usize, hook: NativeScriptFn) {
        if let Ok(mut map) = self.hooks.write() {
            map.insert(script_id, hook);
        }
    }

    /// Fast-path calculation for Pokémon battle damage formula with memoization:
    /// Damage = ((((2 * Level / 5 + 2) * Power * (A / D)) / 50) + 2) * Modifier
    #[inline(always)]
    pub fn eval_damage_fast(&self, level: f64, power: f64, attack: f64, defense: f64, modifier: f64) -> f64 {
        let key = crate::game::damage_memo::DamageKey {
            attacker_id: attack as i32,
            defender_id: defense as i32,
            move_id: power as i32,
            weather: (level as u32 & 0xFF) as u8,
        };
        if let Some(cached) = crate::game::damage_memo::query_damage_memo(key) {
            return (cached as f64 * modifier).max(1.0);
        }
        let base = ((2.0 * level / 5.0 + 2.0) * power * (attack / defense.max(1.0))) / 50.0 + 2.0;
        crate::game::damage_memo::store_damage_memo(key, base as f32);
        (base * modifier).max(1.0)
    }

    /// Fast-path 2D Euclidean distance squared.
    #[inline(always)]
    pub fn distance_squared_fast(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        dx * dx + dy * dy
    }

    /// Fast-path minimum angular difference between two angles in degrees (-180..=180).
    #[inline(always)]
    pub fn angle_difference_fast(&self, a1: f64, a2: f64) -> f64 {
        let diff = (a1 - a2 + 180.0).rem_euclid(360.0) - 180.0;
        if diff < -180.0 { diff + 360.0 } else { diff }
    }

    /// Dispatches a registered native hook if present.
    pub fn dispatch(
        &self,
        script_id: usize,
        game: &mut crate::game::Game,
        context: &mut Context,
        args: &[Value],
    ) -> Option<gml::Result<Value>> {
        if !self.enabled {
            return None;
        }
        if let Ok(map) = self.hooks.read() {
            if let Some(hook) = map.get(&script_id) {
                return Some(hook(game, context, args));
            }
        }
        None
    }
}

static GML_JIT: std::sync::OnceLock<GmlJitEngine> = std::sync::OnceLock::new();

pub fn jit_engine() -> &'static GmlJitEngine {
    GML_JIT.get_or_init(GmlJitEngine::new)
}

fn val_to_f64(val: &Value) -> f64 {
    match val {
        Value::Real(r) => r.into_inner(),
        _ => 0.0,
    }
}

pub fn native_distance_hook(_game: &mut crate::game::Game, _context: &mut Context, args: &[Value]) -> gml::Result<Value> {
    if args.len() >= 4 {
        let x1 = val_to_f64(&args[0]);
        let y1 = val_to_f64(&args[1]);
        let x2 = val_to_f64(&args[2]);
        let y2 = val_to_f64(&args[3]);
        let dx = x2 - x1;
        let dy = y2 - y1;
        Ok(Value::Real(Real::from((dx * dx + dy * dy).sqrt())))
    } else {
        Ok(Value::Real(Real::from(0.0)))
    }
}

pub fn native_angle_hook(_game: &mut crate::game::Game, _context: &mut Context, args: &[Value]) -> gml::Result<Value> {
    if args.len() >= 2 {
        let a1 = val_to_f64(&args[0]);
        let a2 = val_to_f64(&args[1]);
        let diff = (a1 - a2 + 180.0).rem_euclid(360.0) - 180.0;
        let res = if diff < -180.0 { diff + 360.0 } else { diff };
        Ok(Value::Real(Real::from(res)))
    } else {
        Ok(Value::Real(Real::from(0.0)))
    }
}

pub fn native_damage_hook(_game: &mut crate::game::Game, _context: &mut Context, args: &[Value]) -> gml::Result<Value> {
    if args.len() >= 4 {
        let level = val_to_f64(&args[0]);
        let power = val_to_f64(&args[1]);
        let attack = val_to_f64(&args[2]);
        let defense = val_to_f64(&args[3]);
        let modifier = if args.len() >= 5 { val_to_f64(&args[4]) } else { 1.0 };
        let dmg = jit_engine().eval_damage_fast(level, power, attack, defense, modifier);
        Ok(Value::Real(Real::from(dmg)))
    } else {
        Ok(Value::Real(Real::from(1.0)))
    }
}

/// Tries dispatching a registered native fast-path for the given script ID.
pub fn try_fast_path(
    script_id: usize,
    game: &mut crate::game::Game,
    context: &mut Context,
    args: &[Value],
) -> Option<gml::Result<Value>> {
    let engine = jit_engine();
    if !engine.is_enabled() {
        return None;
    }
    if let Some(res) = engine.dispatch(script_id, game, context, args) {
        return Some(res);
    }
    if let Some(Some(script)) = game.assets.scripts.get(script_id) {
        let name_bytes = script.name.0.as_ref();
        if let Ok(name) = std::str::from_utf8(name_bytes) {
            let lower = name.to_ascii_lowercase();
            let hook: Option<NativeScriptFn> = if lower.contains("dano") || lower.contains("daño") || lower.contains("damage") {
                Some(native_damage_hook)
            } else if lower.contains("distancia") || lower.contains("distance") {
                Some(native_distance_hook)
            } else if lower.contains("angulo_diferencia") || lower.contains("angle_difference") {
                Some(native_angle_hook)
            } else {
                None
            };
            if let Some(h) = hook {
                engine.register_hook(script_id, h);
                return Some(h(game, context, args));
            }
        }
    }
    None
}
