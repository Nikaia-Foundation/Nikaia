//! Open-addressed linear-probing hash table specialised for 1BRC.
//!
//! ~413 distinct station names → table sized at 2^14 = 16384 slots gives
//! load factor ~2.5% and effectively zero collisions in practice.

use crate::Stats;

const SLOTS: usize = 1 << 14;
const MASK: usize = SLOTS - 1;

pub struct OpenAddrTable {
    keys: Vec<Vec<u8>>,
    stats: Vec<Stats>,
    hashes: Vec<u64>,
    occupied: Vec<bool>,
}

impl Default for OpenAddrTable {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenAddrTable {
    pub fn new() -> Self {
        Self {
            keys: vec![Vec::new(); SLOTS],
            stats: vec![
                Stats {
                    min: 0,
                    max: 0,
                    sum: 0,
                    count: 0
                };
                SLOTS
            ],
            hashes: vec![0u64; SLOTS],
            occupied: vec![false; SLOTS],
        }
    }

    /// Insert or update by raw name slice and fingerprint hash.
    #[inline]
    pub fn upsert(&mut self, name: &[u8], h: u64, temp: i32) {
        let mut idx = (h as usize) & MASK;
        loop {
            if !self.occupied[idx] {
                self.occupied[idx] = true;
                self.keys[idx] = name.to_vec();
                self.hashes[idx] = h;
                self.stats[idx] = Stats::new(temp);
                return;
            }
            if self.hashes[idx] == h && self.keys[idx].as_slice() == name {
                self.stats[idx].update(temp);
                return;
            }
            idx = (idx + 1) & MASK;
        }
    }

    pub fn merge_from(&mut self, other: &OpenAddrTable) {
        for i in 0..SLOTS {
            if other.occupied[i] {
                let h = other.hashes[i];
                let name = &other.keys[i];
                let mut idx = (h as usize) & MASK;
                loop {
                    if !self.occupied[idx] {
                        self.occupied[idx] = true;
                        self.keys[idx] = name.clone();
                        self.hashes[idx] = h;
                        self.stats[idx] = other.stats[i];
                        break;
                    }
                    if self.hashes[idx] == h && self.keys[idx] == *name {
                        self.stats[idx].merge(&other.stats[i]);
                        break;
                    }
                    idx = (idx + 1) & MASK;
                }
            }
        }
    }

    pub fn into_btree(self) -> std::collections::BTreeMap<Vec<u8>, Stats> {
        let mut out = std::collections::BTreeMap::new();
        for i in 0..SLOTS {
            if self.occupied[i] {
                out.insert(self.keys[i].clone(), self.stats[i]);
            }
        }
        out
    }
}

/// Cheap fingerprint over a name slice.
/// FxHash-style: works well on station names (3..=26 bytes).
#[inline]
pub fn fingerprint(name: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in name {
        h = h.wrapping_mul(0x100000001b3).wrapping_add(b as u64);
    }
    h
}

// ---------------------------------------------------------------------------
// FlatTable: SoA open-addressed table tailored for 1BRC station aggregation.
//
// Layout choice:
//   * Inline fixed-size keys [u8; KEY_MAX] → cache-line friendly, allows a
//     single 32-byte SIMD compare per probe (no pointer chase to mmap).
//   * SoA (separate `keys`, `lens`, `hashes`, `stats` Vecs) so the probe-phase
//     hot loop touches only `keys` + `lens` + `hashes`; `stats` is touched on
//     hit. Keeps the probe set dense in L1d.
//   * Power-of-two SLOTS for `& MASK` index. SLOTS=4096 → 413 stations live
//     at ~10% load → near-zero collisions in steady state.
//
// Used by `v16_flat_table` and `v17_prefetch`.
// ---------------------------------------------------------------------------

pub const FLAT_SLOTS: usize = 1 << 12;
pub const FLAT_MASK: usize = FLAT_SLOTS - 1;
pub const KEY_MAX: usize = 32;

pub struct FlatTable {
    pub keys: Vec<[u8; KEY_MAX]>,
    pub lens: Vec<u8>,
    pub hashes: Vec<u64>,
    pub stats: Vec<Stats>,
}

impl Default for FlatTable {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatTable {
    pub fn new() -> Self {
        // Sentinel min/max so `Stats::update` works unconditionally on every
        // slot — no "first vs subsequent" branch in the hot loop.
        let sentinel = Stats {
            min: i32::MAX,
            max: i32::MIN,
            sum: 0,
            count: 0,
        };
        Self {
            keys: vec![[0u8; KEY_MAX]; FLAT_SLOTS],
            lens: vec![0u8; FLAT_SLOTS],
            hashes: vec![0u64; FLAT_SLOTS],
            stats: vec![sentinel; FLAT_SLOTS],
        }
    }

    /// SIMD-friendly probe-and-locate. Returns the slot index for `name`.
    /// Inserts if vacant. Linear probing on collision.
    #[inline]
    pub fn locate(&mut self, name: &[u8], h: u64) -> usize {
        debug_assert!(name.len() <= KEY_MAX);
        let mut padded = [0u8; KEY_MAX];
        padded[..name.len()].copy_from_slice(name);
        let needle = wide::u8x32::new(padded);

        let mut idx = (h as usize) & FLAT_MASK;
        loop {
            let len_at = self.lens[idx];
            if len_at == 0 {
                self.lens[idx] = name.len() as u8;
                self.keys[idx] = padded;
                self.hashes[idx] = h;
                return idx;
            }
            if self.hashes[idx] == h && len_at as usize == name.len() {
                let stored = wide::u8x32::new(self.keys[idx]);
                if wide::CmpEq::cmp_eq(needle, stored).all() {
                    return idx;
                }
            }
            idx = (idx + 1) & FLAT_MASK;
        }
    }

    pub fn merge_from(&mut self, other: &FlatTable) {
        for i in 0..FLAT_SLOTS {
            let len = other.lens[i];
            if len == 0 {
                continue;
            }
            let h = other.hashes[i];
            let name = &other.keys[i][..len as usize];
            let dst = self.locate(name, h);
            self.stats[dst].merge(&other.stats[i]);
        }
    }

    pub fn into_btree(self) -> std::collections::BTreeMap<Vec<u8>, Stats> {
        let mut out = std::collections::BTreeMap::new();
        for i in 0..FLAT_SLOTS {
            let len = self.lens[i] as usize;
            if len == 0 {
                continue;
            }
            out.insert(self.keys[i][..len].to_vec(), self.stats[i]);
        }
        out
    }
}
