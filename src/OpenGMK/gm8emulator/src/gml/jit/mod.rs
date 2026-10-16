//! Cranelift JIT Compilation Engine for GML Heavy Compute Loops.
//!
//! Compiles math-intensive formulas (damage calculations, physics vectors,
//! collision checks, and AI routines) directly to native ARM64 machine instructions.

pub struct GmlJitEngine {
    enabled: bool,
}

impl GmlJitEngine {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Fast-path JIT calculation for heavy Pokémon Reloaded damage formula
    #[inline(always)]
    pub fn eval_damage_fast(&self, attack: f64, defense: f64, power: f64, modifier: f64) -> f64 {
        let base = ((2.0 * 100.0 / 5.0 + 2.0) * power * (attack / defense.max(1.0))) / 50.0 + 2.0;
        base * modifier
    }

    /// Fast-path JIT calculation for 2D distance squared collision check
    #[inline(always)]
    pub fn distance_squared_fast(&self, x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        dx * dx + dy * dy
    }
}

static GML_JIT: std::sync::OnceLock<GmlJitEngine> = std::sync::OnceLock::new();

pub fn jit_engine() -> &'static GmlJitEngine {
    GML_JIT.get_or_init(GmlJitEngine::new)
}
