// The fastest Rust entry for k-nucleotide in the Computer Language Benchmarks
// Game on 2026-11-23 that links no unmaintained crate, "Rust entry 3" (3.28 s,
// third of the Rust entries):
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/program/knucleotide-rust-3.html
// Entry 7 (2.57 s) runs on tokio-threadpool 0.1 and entry 6 (2.79 s) hashes
// with fxhash, both unmaintained (RUSTSEC-2026-0051, RUSTSEC-2025-0057, #2806).
// Copied unchanged below this comment. It is published under the 3-Clause BSD
// License of the Benchmarks Game:
// https://benchmarksgame-team.pages.debian.net/benchmarksgame/license.html
// The game builds it with `rustc -C opt-level=3 -C target-cpu=ivybridge
// -C codegen-units=1` against hashbrown (whose default hasher, foldhash since
// 0.15, is seeded per process); `benches/game/game.nika` does the same
// through Cargo. It counts the five longer fragment lengths on five threads
// of `std::thread` and the lengths 1 and 2 on the main thread; callgrind
// serialises the threads and counts them all.


// The Computer Language Benchmarks Game
// https://salsa.debian.org/benchmarksgame-team/benchmarksgame/
//
// contributed by Tom Kaitchuck

// Based on k-nucleotide Rust #7
// Switched to used Hashbrown and removed custom hash code.
// Removed rayon and use threads directly
// Copied the read_input function from k-nucleotide Rust #4

extern crate hashbrown;

use std::io::BufRead;
use std::sync::Arc;
use hashbrown::HashMap;
use std::thread;

type Map = HashMap<Code, u32>;

#[derive(Hash, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
struct Code(u64);
impl Code {
    fn push(&mut self, c: u8, mask: u64) {
        self.0 <<= 2;
        self.0 |= c as u64;
        self.0 &= mask;
    }
    fn from_str(s: &str) -> Code {
        let mask = Code::make_mask(s.len());
        let mut res = Code(0);
        for c in s.as_bytes() {
            res.push(Code::encode_byte(*c), mask);
        }
        res
    }
    fn to_string(&self, frame: usize) -> String {
        let mut res = vec![];
        let mut code = self.0;
        for _ in 0..frame {
            let c = match code as u8 & 0b11 {
                c if c == Code::encode_byte(b'A') => b'A',
                c if c == Code::encode_byte(b'T') => b'T',
                c if c == Code::encode_byte(b'G') => b'G',
                c if c == Code::encode_byte(b'C') => b'C',
                _ => unreachable!(),
            };
            res.push(c);
            code >>= 2;
        }
        res.reverse();
        String::from_utf8(res).unwrap()
    }
    fn make_mask(frame: usize) -> u64 {
        (1u64 << (2 * frame)) - 1
    }
    #[inline(always)]
    fn encode_byte(c: u8) -> u8 {
        (c & 0b110) >> 1
    }
}

struct Iter<'a> {
    iter: std::slice::Iter<'a, u8>,
    code: Code,
    mask: u64,
}
impl<'a> Iter<'a> {
    fn new(input: &[u8], frame: usize) -> Iter {
        let mut iter = input.iter();
        let mut code = Code(0);
        let mask = Code::make_mask(frame);
        for c in iter.by_ref().take(frame - 1) {
            code.push(*c, mask);
        }
        Iter {
            iter: iter,
            code: code,
            mask: mask,
        }
    }
}
impl<'a> Iterator for Iter<'a> {
    type Item = Code;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|&c| {
            self.code.push(c, self.mask);
            self.code
        })
    }
}

fn gen_freq(input: &[u8], frame: usize) -> Map {
    let mut freq = Map::default();
    for code in Iter::new(input, frame) {
        *freq.entry(code).or_insert(0) += 1;
    }
    freq
}

#[derive(Clone, Copy)]
struct Freq(usize);
#[derive(Clone, Copy)]
struct Occ(&'static str);

impl Freq {
    fn print(&self, freq: &Map) {
        let mut v: Vec<_> = freq.iter()
                                .map(|(&code, &count)| (count, code))
                                .collect();
        v.sort();
        let total = v.iter().map(|&(count, _)| count).sum::<u32>() as f32;
        for &(count, key) in v.iter().rev() {
            println!("{} {:.3}", 
            key.to_string(self.0), (count as f32 * 100.) / total);
        }
        println!("");
    }
}
impl Occ {
    fn print(&self, freq: &Map) {
        let count = if freq.contains_key(&Code::from_str(self.0)) {
            freq[&Code::from_str(self.0)]
        } else { 
            0 
        };
        println!("{}\t{}", count, self.0);                                
    }
}

fn read_input() -> Vec<u8> {
    let stdin = std::io::stdin();
    let mut r = stdin.lock();
    let key = b">THREE";
    let mut res = Vec::with_capacity(65536);
    let mut line = Vec::with_capacity(64);

    loop {
        match r.read_until(b'\n', &mut line) {
            Ok(b) if b > 0 => if line.starts_with(key) { break },
            _ => break,
        }
        line.clear();
    }

    loop {
        line.clear();
        match r.read_until(b'\n', &mut line) {
            Ok(b) if b > 0 => 
                res.extend(line[..line.len()-1].iter()
                   .cloned().map(Code::encode_byte)),
            _ => break,
        }
    }
    res
}

fn main() {
    let occs = vec![
        Occ("GGTATTTTAATTTATAGT"),
        Occ("GGTATTTTAATT"),
        Occ("GGTATT"),
        Occ("GGTA"),
        Occ("GGT"),
    ];
    let input = Arc::new(read_input());

    // In reverse to spawn big tasks first
    let results : Vec<_> = occs.into_iter().map(|item| {
        let input = input.clone();
        thread::spawn(move || (item, gen_freq(&input, item.0.len())) )
    }).collect();

    Freq(1).print(&gen_freq(&input, 1));
    Freq(2).print(&gen_freq(&input, 2));

    for t in results.into_iter().rev() {
        let (item, freq) = t.join().unwrap();
        item.print(&freq);
    }
}
