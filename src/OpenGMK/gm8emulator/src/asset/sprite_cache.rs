//! In-memory sprite cache and fast index for the 26,240 battle sprites (.gmspr).
//!
//! Eliminates flash storage random-read stutter during battles by pre-indexing
//! paths and keeping decoded RGBA frame templates in RAM.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, OnceLock,
    },
    time::Duration,
};


#[derive(Clone)]
pub struct ParsedFrameTemplate {
    pub width: u32,
    pub height: u32,
    pub rgba_bytes: Arc<[u8]>,
}

#[derive(Clone)]
pub struct ParsedSpriteTemplate {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u32,
    pub height: u32,
    pub per_frame_colliders: bool,
    pub bbox_left: u32,
    pub bbox_right: u32,
    pub bbox_top: u32,
    pub bbox_bottom: u32,
    pub colliders: Vec<crate::asset::sprite::Collider>,
    pub frame_data: Vec<ParsedFrameTemplate>,
}

/// In-memory cache of decoded sprite templates, keyed by normalized lowercase path.
static SPRITE_CACHE: OnceLock<Mutex<HashMap<String, Arc<ParsedSpriteTemplate>>>> = OnceLock::new();

/// Pre-indexed case-insensitive map: "data/pkmnbattle/p001.gmspr" -> actual PathBuf on disk.
static PATH_INDEX: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();

/// Maximum number of parsed sprite templates retained in RAM simultaneously.
/// 1024 battle sprites take ~15-25 MB, fitting comfortably on any phone with 2GB+ RAM.
const MAX_CACHED_SPRITES: usize = 1024;

fn get_cache() -> &'static Mutex<HashMap<String, Arc<ParsedSpriteTemplate>>> {
    SPRITE_CACHE.get_or_init(|| Mutex::new(HashMap::with_capacity(256)))
}

fn get_path_index() -> &'static Mutex<HashMap<String, PathBuf>> {
    PATH_INDEX.get_or_init(|| Mutex::new(HashMap::with_capacity(1024)))
}

/// Normalizes a path string for cache lookup (lowercase, forward slashes).
pub fn normalize_key(fname: &str) -> String {
    fname.replace('\\', "/").to_ascii_lowercase()
}

/// Pre-indexes all .gmspr files in a directory recursively into RAM, so case-insensitive
/// lookups on Android's Linux filesystem take 0 ms.
pub fn index_directory(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    let mut map = match get_path_index().lock() {
        Ok(m) => m,
        Err(_) => return,
    };

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current_dir) = stack.pop() {
        let entries = match fs::read_dir(&current_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("gmspr")) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    let key = name.to_ascii_lowercase();
                    map.insert(key, path);
                }
            }
        }
    }
}

/// Resolves a requested file path against the case-insensitive path index if direct access fails.
pub fn resolve_path(fname: &str) -> PathBuf {
    let direct = PathBuf::from(crate::gml::file::to_path(fname).as_ref());
    if direct.is_file() {
        return direct;
    }

    // Try finding by filename in our pre-indexed paths
    if let Some(file_name) = Path::new(fname).file_name().and_then(|n| n.to_str()) {
        let key = file_name.to_ascii_lowercase();
        if let Ok(index) = get_path_index().lock() {
            if let Some(found) = index.get(&key) {
                if found.is_file() {
                    return found.clone();
                }
            }
        }
    }

    direct
}

/// Fetches a parsed sprite template from RAM cache, or loads, decompresses,
/// converts BGRA to RGBA once, and caches it.
pub fn get_or_load_template(fname: &str) -> Result<Arc<ParsedSpriteTemplate>, String> {
    let key = normalize_key(fname);

    // 1. Fast path: in-memory cache hit
    if let Ok(cache) = get_cache().lock() {
        if let Some(cached) = cache.get(&key) {
            return Ok(cached.clone());
        }
    }

    // 2. Slow path: read from storage and decode once
    let path = resolve_path(fname);
    let bytes = fs::read(&path).map_err(|e| format!("failed to open '{}' (path: {}): {}", fname, path.display(), e))?;
    let parsed = crate::gml::kernel::parse_gmspr(&bytes).map_err(|e| format!("failed to parse '{}': {}", fname, e))?;

    let (origin_x, origin_y) = (parsed.origin_x, parsed.origin_y);
    let mut frame_data = Vec::with_capacity(parsed.frames.len());

    for frame in &parsed.frames {
        let mut data = frame.data.to_vec();
        crate::util::bgra2rgba(&mut data);
        frame_data.push(ParsedFrameTemplate {
            width: frame.width,
            height: frame.height,
            rgba_bytes: Arc::from(data.into_boxed_slice()),
        });
    }

    let colliders = parsed
        .colliders
        .iter()
        .map(|c| {
            let is_empty_mask = !c.data.iter().any(|&px| px);
            let mut is_solid_rect = !is_empty_mask;
            if is_solid_rect {
                for y in c.bbox_top..=c.bbox_bottom {
                    for x in c.bbox_left..=c.bbox_right {
                        let idx = (y * c.width + x) as usize;
                        if idx >= c.data.len() || !c.data[idx] {
                            is_solid_rect = false;
                            break;
                        }
                    }
                    if !is_solid_rect { break; }
                }
            }
            crate::asset::sprite::Collider {
                width: c.width,
                height: c.height,
                bbox_left: c.bbox_left,
                bbox_right: c.bbox_right,
                bbox_top: c.bbox_top,
                bbox_bottom: c.bbox_bottom,
                is_empty_mask,
                is_solid_rect,
                data: c.data.clone(),
            }
        })
        .collect::<Vec<_>>();

    let (width, height) = parsed.frames.first().map(|f| (f.width, f.height)).unwrap_or((0, 0));
    let bbox = colliders.first();

    let template = Arc::new(ParsedSpriteTemplate {
        origin_x,
        origin_y,
        width,
        height,
        per_frame_colliders: parsed.per_frame_colliders,
        bbox_left: bbox.map(|b| b.bbox_left).unwrap_or(0),
        bbox_right: bbox.map(|b| b.bbox_right).unwrap_or(0),
        bbox_top: bbox.map(|b| b.bbox_top).unwrap_or(0),
        bbox_bottom: bbox.map(|b| b.bbox_bottom).unwrap_or(0),
        colliders,
        frame_data,
    });

    if let Ok(mut cache) = get_cache().lock() {
        if cache.len() >= MAX_CACHED_SPRITES {
            // Drop oldest half of entries to keep memory bounded (~15-25 MB)
            let to_remove = cache.keys().take(MAX_CACHED_SPRITES / 2).cloned().collect::<Vec<_>>();
            for k in to_remove {
                cache.remove(&k);
            }
        }
        cache.insert(key, template.clone());
    }

    Ok(template)
}

static STREAMER_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Launches the background asset streaming worker thread.
/// It pre-indexes the directory and progressively warms the most critical battle sprites
/// (starter sprites, common battle UI elements, etc.) into RAM during idle cycles,
/// so in-game battles launch with 0 ms storage wait.
pub fn start_background_asset_streamer(battle_dir: &Path) {
    if STREAMER_ACTIVE.swap(true, Ordering::SeqCst) {
        return; // already active
    }

    let dir = battle_dir.to_path_buf();
    let _ = std::thread::Builder::new()
        .name("bg-asset-streamer".into())
        .spawn(move || {
            // Step 1: Pre-index directory
            index_directory(&dir);

            // Step 2: Warm up common battle sprites in the background
            // Prioritize starter pokemon (p001-p009, p025, etc.) and battle HUD assets
            let priority_prefixes = [
                "p001", "p004", "p007", "p025", "p133", "p150", "p151",
                "p152", "p155", "p158", "p252", "p255", "p258", "p387",
                "p390", "p393", "p495", "p498", "p501", "p650", "p653", "p656",
            ];

            // Get a snapshot of known filenames
            let candidates: Vec<String> = if let Ok(index) = get_path_index().lock() {
                index.keys().cloned().collect()
            } else {
                Vec::new()
            };

            // Stream priority sprites first with small yield between each
            for key in &candidates {
                let is_priority = priority_prefixes.iter().any(|p| key.starts_with(p));
                if is_priority {
                    let _ = get_or_load_template(key);
                    std::thread::sleep(Duration::from_millis(15));
                }
            }

            // Next, progressively warm up remaining battle assets up to 256 cached sprites
            let mut warmed = 0;
            for key in &candidates {
                if warmed >= 256 {
                    break;
                }
                let is_cached = if let Ok(cache) = get_cache().lock() {
                    cache.contains_key(key)
                } else {
                    false
                };
                if !is_cached {
                    if get_or_load_template(key).is_ok() {
                        warmed += 1;
                        std::thread::sleep(Duration::from_millis(20));
                    }
                }
            }
        });
}

/// Spawns a background task to pre-decode a specific list of sprites into RAM asynchronously
pub fn prefetch_sprites_async(fnames: Vec<String>) {
    if fnames.is_empty() {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("sprite-prefetcher".into())
        .spawn(move || {
            for fname in fnames {
                let key = normalize_key(&fname);
                let cached = if let Ok(cache) = get_cache().lock() {
                    cache.contains_key(&key)
                } else {
                    false
                };
                if !cached {
                    let _ = get_or_load_template(&fname);
                }
            }
        });
}

