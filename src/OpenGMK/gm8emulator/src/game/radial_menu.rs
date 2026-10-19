//! Tactical Combat Radial Menu with Real-Time Time Dilation (0.2x speed).
//!
//! Provides a seamless circular HUD overlay allowing players to switch Pokémon,
//! consume berries/potions, or trigger quick Pokéballs in < 1s during real-time combat
//! without opening the backpack or pausing gameplay awkwardly.

use std::sync::atomic::{AtomicBool, Ordering};

pub static RADIAL_MENU_ENABLED: AtomicBool = AtomicBool::new(false);
pub static RADIAL_MENU_ACTIVE: AtomicBool = AtomicBool::new(false);

#[inline(always)]
pub fn is_radial_menu_enabled() -> bool {
    RADIAL_MENU_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_radial_menu_enabled(enabled: bool) {
    RADIAL_MENU_ENABLED.store(enabled, Ordering::Relaxed);
}

#[inline(always)]
pub fn is_radial_menu_active() -> bool {
    RADIAL_MENU_ACTIVE.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_radial_menu_active(active: bool) {
    RADIAL_MENU_ACTIVE.store(active, Ordering::Relaxed);
}

/// Returns the current time scale multiplier for the game engine.
/// When the radial menu is open, the game slows down to 0.2x (Time Dilation).
#[inline(always)]
pub fn get_combat_time_dilation() -> f32 {
    if is_radial_menu_active() {
        0.20 // 20% slow motion
    } else {
        1.00 // Normal speed
    }
}

/// Radial Menu Sectors (6 tactical slices).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum RadialSector {
    None = 0,
    SwitchPokemon = 1, // Slice 1 (Top)
    HealHp = 2,        // Slice 2 (Top-Right)
    CureStatus = 3,    // Slice 3 (Bottom-Right)
    QuickBall = 4,     // Slice 4 (Bottom)
    SpecialMove = 5,   // Slice 5 (Bottom-Left)
    TacticalRun = 6,   // Slice 6 (Top-Left)
}

pub struct RadialMenuState {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
    pub selected_sector: RadialSector,
}

impl Default for RadialMenuState {
    fn default() -> Self {
        Self {
            center_x: 0.5,
            center_y: 0.5,
            radius: 0.25,
            selected_sector: RadialSector::None,
        }
    }
}

impl RadialMenuState {
    /// Evaluates touch coordinates (normalized 0.0..1.0) and determines active sector.
    pub fn update_touch(&mut self, touch_x: f32, touch_y: f32) -> RadialSector {
        let dx = touch_x - self.center_x;
        let dy = touch_y - self.center_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 0.05 || dist > self.radius * 1.5 {
            self.selected_sector = RadialSector::None;
            return RadialSector::None;
        }

        // Angle in degrees: 0 is right, 90 is down, 180 is left, 270 is up
        let mut deg = dy.atan2(dx).to_degrees();
        if deg < 0.0 {
            deg += 360.0;
        }

        // Shift by 90 degrees so top is 0
        let shifted = (deg + 90.0) % 360.0;
        let sector_index = (shifted / 60.0).floor() as u8;

        self.selected_sector = match sector_index {
            0 => RadialSector::SwitchPokemon,
            1 => RadialSector::HealHp,
            2 => RadialSector::CureStatus,
            3 => RadialSector::QuickBall,
            4 => RadialSector::SpecialMove,
            5 => RadialSector::TacticalRun,
            _ => RadialSector::None,
        };

        self.selected_sector
    }
}
