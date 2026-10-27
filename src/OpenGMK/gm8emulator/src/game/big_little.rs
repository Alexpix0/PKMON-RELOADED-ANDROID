//! CPU Big.LITTLE Core Topology Detection and Thread Affinity Pinning.
//!
//! Evaluates processor frequencies in `/sys/devices/system/cpu/cpu*/cpufreq/cpuinfo_max_freq`
//! to identify Prime/Performance (Big) cores vs Efficiency (LITTLE) cores.
//! Pins logic/render threads to Big cores and audio/workers to LITTLE cores via `sched_setaffinity`.

use std::sync::atomic::{AtomicBool, Ordering};

pub static BIG_LITTLE_AFFINITY_ENABLED: AtomicBool = AtomicBool::new(true);

#[inline(always)]
pub fn is_big_little_affinity_enabled() -> bool {
    BIG_LITTLE_AFFINITY_ENABLED.load(Ordering::Relaxed)
}

#[inline(always)]
pub fn set_big_little_affinity_enabled(enabled: bool) {
    BIG_LITTLE_AFFINITY_ENABLED.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Debug, Default)]
pub struct CpuTopology {
    pub big_cores: Vec<usize>,
    pub little_cores: Vec<usize>,
    pub is_free_scheduler: bool,
}

impl CpuTopology {
    /// Discovers system CPU topology on Linux/Android.
    /// Handles asymmetric Big.LITTLE (A7x vs A5x) as well as homogeneous
    /// multi-gear Cortex-A53/A55 architectures (like MediaTek Helio G25: 4x 2.0 GHz + 4x 1.5 GHz).
    pub fn discover() -> Self {
        let mut cores_with_freq = Vec::new();

        #[cfg(target_os = "android")]
        {
            for core_id in 0..16 {
                let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/cpuinfo_max_freq", core_id);
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(freq) = content.trim().parse::<u64>() {
                        cores_with_freq.push((core_id, freq));
                    }
                }
            }
        }

        if cores_with_freq.is_empty() {
            // Sysfs cpufreq inaccessible or blocked by OEM sandbox:
            // Operate in free Linux scheduler mode to avoid erratic affinity pinning
            return Self {
                big_cores: Vec::new(),
                little_cores: Vec::new(),
                is_free_scheduler: true,
            };
        }

        let min_freq = cores_with_freq.iter().map(|&(_, f)| f).min().unwrap_or(0);
        let max_freq = cores_with_freq.iter().map(|&(_, f)| f).max().unwrap_or(0);

        // If all cores have identical or nearly identical frequencies (<5% difference),
        // we have a single-frequency homogeneous cluster: operate in free scheduler mode.
        if min_freq == 0 || max_freq == 0 || max_freq <= min_freq || (max_freq - min_freq) < (min_freq / 20) {
            return Self {
                big_cores: Vec::new(),
                little_cores: Vec::new(),
                is_free_scheduler: true,
            };
        }

        // Midpoint threshold between frequency clusters (e.g., 1.5 GHz & 2.0 GHz -> 1.75 GHz)
        let threshold = min_freq + (max_freq - min_freq) / 2;

        let mut big = Vec::new();
        let mut little = Vec::new();

        for (id, freq) in cores_with_freq {
            if freq > threshold {
                big.push(id);
            } else {
                little.push(id);
            }
        }

        if big.is_empty() || little.is_empty() {
            Self {
                big_cores: Vec::new(),
                little_cores: Vec::new(),
                is_free_scheduler: true,
            }
        } else {
            Self {
                big_cores: big,
                little_cores: little,
                is_free_scheduler: false,
            }
        }
    }

    /// Pins the current calling thread to high-performance Big cores.
    pub fn pin_current_thread_to_big(&self) {
        if !self.big_cores.is_empty() {
            self.apply_affinity(&self.big_cores);
        }
    }

    /// Pins the current calling thread to energy-efficient LITTLE cores.
    pub fn pin_current_thread_to_little(&self) {
        if self.is_free_scheduler || !is_big_little_affinity_enabled() || self.little_cores.is_empty() {
            return;
        }
        self.apply_affinity(&self.little_cores);
    }

    fn apply_affinity(&self, core_ids: &[usize]) {
        #[cfg(target_os = "android")]
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            // libc::CPU_SET macro implementation
            for &id in core_ids {
                if id < 1024 {
                    let idx = id / (8 * std::mem::size_of::<libc::c_ulong>());
                    let bit = id % (8 * std::mem::size_of::<libc::c_ulong>());
                    let ptr = &mut set as *mut _ as *mut libc::c_ulong;
                    *ptr.add(idx) |= 1 << bit;
                }
            }
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
        #[cfg(not(target_os = "android"))]
        let _ = core_ids;
    }
}

static GLOBAL_TOPOLOGY: std::sync::LazyLock<CpuTopology> =
    std::sync::LazyLock::new(CpuTopology::discover);

pub fn get_cpu_topology() -> &'static CpuTopology {
    &GLOBAL_TOPOLOGY
}
