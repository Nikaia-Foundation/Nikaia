//! v16_flat_table — custom open-addressed flat hashtable with inline
//! `[u8; 32]` keys + 32-byte SIMD key comparison.
//!
//! Hypothesis: replace `hashbrown::HashMap<&[u8], Stats, FxBuildHasher>`
//! (v15) with a SoA table where each slot stores its key inline. The probe
//! loop becomes pure linear scan over dense `lens`/`hashes`/`keys` arrays;
//! key equality is a single `u8x32::cmp_eq().all()`.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use memmap2::Mmap;
use rust_1brc::hash::FlatTable;
use rust_1brc::{chunk_bounds, format_output, parse_temp_merykitty};
use rustc_hash::FxBuildHasher;
use std::fs::File;
use std::hash::{BuildHasher, Hasher};
use std::thread;
use wide::{CmpEq, u8x32};

const LANE: usize = 32;

#[inline]
fn fxhash(bytes: &[u8]) -> u64 {
    let mut h = FxBuildHasher.build_hasher();
    h.write(bytes);
    h.finish()
}

#[inline]
fn find_semi(hay: &[u8]) -> Option<usize> {
    let needle = u8x32::splat(b';');
    let mut chunks = hay.chunks_exact(LANE);
    let simd_hit = chunks.by_ref().enumerate().find_map(|(i, c)| {
        let bits = u8x32::new(c.try_into().unwrap()).cmp_eq(needle).move_mask() as u32;
        (bits != 0).then(|| i * LANE + bits.trailing_zeros() as usize)
    });
    simd_hit.or_else(|| {
        let off = hay.len() - chunks.remainder().len();
        chunks
            .remainder()
            .iter()
            .position(|&b| b == b';')
            .map(|p| off + p)
    })
}

fn process_chunk(data: &[u8], start: usize, end: usize) -> FlatTable {
    let mut t = FlatTable::new();
    let mut i = start;
    while i < end {
        let semi = i + find_semi(&data[i..end]).expect("semi");
        let name = &data[i..semi];
        let h = fxhash(name);
        let (temp, consumed) = parse_temp_merykitty(&data[semi + 1..]);
        let idx = t.locate(name, h);
        t.stats[idx].update(temp);
        i = semi + 1 + consumed;
    }
    t
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: v16_flat_table <file>");
    let mmap = unsafe { Mmap::map(&File::open(&path).expect("open")).expect("mmap") };
    let data: &[u8] = &mmap;

    let n = thread::available_parallelism().map_or(1, |n| n.get());
    let bounds = chunk_bounds(data, n);

    let tables: Vec<FlatTable> = thread::scope(|s| {
        bounds
            .iter()
            .map(|&(a, b)| s.spawn(move || process_chunk(data, a, b)))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });

    let merged = tables.into_iter().fold(FlatTable::new(), |mut acc, t| {
        acc.merge_from(&t);
        acc
    });
    println!("{}", format_output(&merged.into_btree()));
}
