//! In-Game Retro Achievements Engine with 60 FPS Animated Popups.
//!
//! Evaluates game event milestones:
//! - Special Gym Leader challenges.
//! - Shiny Pokémon captures.
//! - No-damage battle victories.
//! - Legendary Pokémon encounters.
//!
//! Dispatches smooth animated top-banner popups with sound pulses.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

pub static ACHIEVEMENTS_ENABLED: AtomicBool = AtomicBool::new(true);

#[inline(always)]
pub fn is_achievements_enabled() -> bool {
    ACHIEVEMENTS_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_achievements_enabled(enabled: bool) {
    ACHIEVEMENTS_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Debug)]
pub struct Achievement {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub unlocked: bool,
}

pub struct ActiveBanner {
    pub title: &'static str,
    pub description: &'static str,
    pub timer_frames: u32,
    pub alpha: f32,
}

pub struct AchievementManager {
    pub achievements: Vec<Achievement>,
    pub pending_banners: Vec<ActiveBanner>,
}

impl Default for AchievementManager {
    fn default() -> Self {
        Self {
            achievements: vec![
                Achievement {
                    id: "first_shiny",
                    title: "Destello Legendario",
                    description: "Encuentra tu primer Pokémon Variocolor (Shiny)",
                    unlocked: false,
                },
                Achievement {
                    id: "brock_pure",
                    title: "Roca Sólida",
                    description: "Derrota a Brock sin que ningún Pokémon de tu equipo caiga",
                    unlocked: false,
                },
                Achievement {
                    id: "speed_demon",
                    title: "Velocidad Supersónica",
                    description: "Gana un combate en tiempo real en menos de 10 segundos",
                    unlocked: false,
                },
                Achievement {
                    id: "master_catcher",
                    title: "Coleccionista Nato",
                    description: "Registra más de 50 especies en tu Pokédex",
                    unlocked: false,
                },
            ],
            pending_banners: Vec::new(),
        }
    }
}

static GLOBAL_ACHIEVEMENTS: std::sync::LazyLock<Mutex<AchievementManager>> =
    std::sync::LazyLock::new(|| Mutex::new(AchievementManager::default()));

pub fn unlock_achievement(id: &str) {
    if !is_achievements_enabled() {
        return;
    }

    if let Ok(mut mgr) = GLOBAL_ACHIEVEMENTS.lock() {
        for ach in mgr.achievements.iter_mut() {
            if ach.id == id && !ach.unlocked {
                ach.unlocked = true;
                mgr.pending_banners.push(ActiveBanner {
                    title: ach.title,
                    description: ach.description,
                    timer_frames: 180, // 3 seconds at 60 FPS
                    alpha: 1.0,
                });
                break;
            }
        }
    }
}

pub fn update_achievement_banners() -> Option<(&'static str, &'static str, f32)> {
    if !is_achievements_enabled() {
        return None;
    }

    if let Ok(mut mgr) = GLOBAL_ACHIEVEMENTS.lock() {
        if let Some(banner) = mgr.pending_banners.first_mut() {
            banner.timer_frames = banner.timer_frames.saturating_sub(1);
            if banner.timer_frames < 30 {
                banner.alpha = (banner.timer_frames as f32) / 30.0;
            }
            let res = (banner.title, banner.description, banner.alpha);
            if banner.timer_frames == 0 {
                mgr.pending_banners.remove(0);
            }
            return Some(res);
        }
    }
    None
}
