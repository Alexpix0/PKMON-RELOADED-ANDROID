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
}

impl CpuTopology {
    /// Discovers system CPU topology on Linux/Android.
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
            // Default assumption if sysfs is blocked by OEM sandbox:
            // Octa-core: 0-3 Little (A55), 4-7 Big (A75/A76/X)
            return Self {
                big_cores: vec![4, 5, 6, 7],
                little_cores: vec![0, 1, 2, 3],
            };
        }

        // Sort by frequency ascending to find median split
        cores_with_freq.sort_by_key(|&(_, freq)| freq);
        let median_freq = cores_with_freq[cores_with_freq.len() / 2].1;

        let mut big = Vec::new();
        let mut little = Vec::new();

        for (id, freq) in cores_with_freq {
            if freq >= median_freq {
                big.push(id);
            } else {
                little.push(id);
            }
        }

        Self {
            big_cores: if big.is_empty() { vec![0] } else { big },
            little_cores: if little.is_empty() { vec![0] } else { little },
        }
    }

    /// Pins the current calling thread to high-performance Big cores.
    pub fn pin_current_thread_to_big(&self) {
        if !is_big_little_affinity_enabled() || self.big_cores.is_empty() {
            return;
        }
        self.apply_affinity(&self.big_cores);
    }

    /// Pins the current calling thread to energy-efficient LITTLE cores.
    pub fn pin_current_thread_to_little(&self) {
        if !is_big_little_affinity_enabled() || self.little_cores.is_empty() {
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
