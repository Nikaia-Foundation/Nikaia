//! v20_safe — fully safe Rust implementation for the complete 1BRC input contract.
//!
//! Unlike the mmap-based variants, this binary copies the input through safe
//! `std::io` APIs. It accepts arbitrary UTF-8 station names up to 100 bytes,
//! at most 10,000 distinct stations, and does not require tail padding.

#![forbid(unsafe_code)]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use hashbrown::HashMap;
use hashbrown::hash_map::RawEntryMut;
use memchr::memchr;
use rapidhash::fast::RandomState;
#[cfg(any(test, not(unix)))]
use rust_1brc::chunk_bounds;
use rust_1brc::{Stats, format_output};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::hash::BuildHasher;
#[cfg(not(unix))]
use std::io::Read;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;

const MAX_NAME_LEN: usize = 100;
const MAX_STATIONS: usize = 10_000;
const INITIAL_CAPACITY: usize = 512;
const MAX_RECORD_LEN: usize = MAX_NAME_LEN + 7;
const BLOCK_SIZE: usize = 4 * 1024 * 1024;

type Map = HashMap<Box<[u8]>, Stats, RandomState>;

#[derive(Debug)]
enum AppError {
    Usage,
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    InvalidRecord {
        offset: usize,
        reason: &'static str,
    },
    TooManyStations,
    InputChanged {
        path: PathBuf,
    },
    ThreadSpawn(io::Error),
    WorkerPanicked,
}

impl AppError {
    fn invalid(offset: usize, reason: &'static str) -> Self {
        Self::InvalidRecord { offset, reason }
    }

    fn exit_code(&self) -> u8 {
        match self {
            Self::Usage | Self::InvalidRecord { .. } | Self::TooManyStations => 2,
            Self::Io { .. }
            | Self::InputChanged { .. }
            | Self::ThreadSpawn(_)
            | Self::WorkerPanicked => 1,
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(f, "usage: v20_safe <file>"),
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "{operation} {}: {source}", path.display()),
            Self::InvalidRecord { offset, reason } => {
                write!(f, "invalid record at byte {offset}: {reason}")
            }
            Self::TooManyStations => write!(
                f,
                "input contains more than {MAX_STATIONS} distinct station names"
            ),
            Self::InputChanged { path } => write!(
                f,
                "{} changed size while it was being read; retry with an immutable input",
                path.display()
            ),
            Self::ThreadSpawn(source) => write!(f, "spawning a worker thread: {source}"),
            Self::WorkerPanicked => write!(f, "a worker thread terminated unexpectedly"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } | Self::ThreadSpawn(source) => Some(source),
            _ => None,
        }
    }
}

fn io_error(operation: &'static str, path: &Path, source: io::Error) -> AppError {
    AppError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(not(unix))]
fn read_input(path: &Path) -> Result<Vec<u8>, AppError> {
    let mut file = File::open(path).map_err(|e| io_error("opening", path, e))?;
    let before = file
        .metadata()
        .map_err(|e| io_error("reading metadata for", path, e))?
        .len();
    let capacity = usize::try_from(before)
        .map_err(|_| AppError::invalid(0, "file is too large for this target architecture"))?;

    let mut data = Vec::with_capacity(capacity);
    file.read_to_end(&mut data)
        .map_err(|e| io_error("reading", path, e))?;
    let after = file
        .metadata()
        .map_err(|e| io_error("re-reading metadata for", path, e))?
        .len();

    if before != after || after != data.len() as u64 {
        return Err(AppError::InputChanged {
            path: path.to_path_buf(),
        });
    }
    Ok(data)
}

#[inline]
fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

/// Parses one temperature and returns `(tenths, bytes including newline)`.
///
/// Slice patterns make every access bounds-checked while the guards validate
/// all digits. The accepted language is exactly X.Y, XX.Y, -X.Y, or -XX.Y.
#[inline]
fn parse_temperature(bytes: &[u8], offset: usize) -> Result<(i32, usize), AppError> {
    let parsed = match bytes {
        // XX.Y first: roughly three quarters of the canonical dataset.
        [a, b, b'.', c, b'\n', ..] if is_digit(*a) && is_digit(*b) && is_digit(*c) => (
            ((a - b'0') as i32) * 100 + ((b - b'0') as i32) * 10 + (c - b'0') as i32,
            5,
        ),
        [a, b'.', c, b'\n', ..] if is_digit(*a) && is_digit(*c) => {
            (((a - b'0') as i32) * 10 + (c - b'0') as i32, 4)
        }
        [b'-', a, b'.', c, b'\n', ..] if is_digit(*a) && is_digit(*c) => {
            (-(((a - b'0') as i32) * 10 + (c - b'0') as i32), 5)
        }
        [b'-', a, b, b'.', c, b'\n', ..] if is_digit(*a) && is_digit(*b) && is_digit(*c) => (
            -(((a - b'0') as i32) * 100 + ((b - b'0') as i32) * 10 + (c - b'0') as i32),
            6,
        ),
        _ => {
            return Err(AppError::invalid(
                offset,
                "invalid temperature or missing newline",
            ));
        }
    };
    Ok(parsed)
}

#[inline]
fn hash_name(state: &RandomState, name: &[u8]) -> u64 {
    state.hash_one(name)
}

fn validate_name(name: &[u8], offset: usize) -> Result<(), AppError> {
    std::str::from_utf8(name)
        .map_err(|_| AppError::invalid(offset, "station name is not valid UTF-8"))?;
    Ok(())
}

#[inline]
fn upsert(map: &mut Map, name: &[u8], temp: i32, offset: usize) -> Result<(), AppError> {
    let hash = hash_name(map.hasher(), name);
    let at_station_limit = map_len_at_limit(map.len());
    match map
        .raw_entry_mut()
        .from_hash(hash, |stored| stored.as_ref() == name)
    {
        RawEntryMut::Occupied(mut entry) => entry.get_mut().update(temp),
        RawEntryMut::Vacant(entry) => {
            if at_station_limit {
                return Err(AppError::TooManyStations);
            }
            validate_name(name, offset)?;
            entry.insert_hashed_nocheck(hash, name.into(), Stats::new(temp));
        }
    }
    Ok(())
}

#[inline]
fn map_len_at_limit(len: usize) -> bool {
    len >= MAX_STATIONS
}

fn process_slice(
    map: &mut Map,
    data: &[u8],
    start: usize,
    end: usize,
    absolute_base: usize,
) -> Result<(), AppError> {
    let mut i = start;
    while i < end {
        let search_end = (i + MAX_NAME_LEN + 1).min(end);
        let Some(relative_semi) = memchr(b';', &data[i..search_end]) else {
            return Err(AppError::invalid(
                absolute_base + i,
                "station name is longer than 100 bytes or missing ';'",
            ));
        };
        if relative_semi == 0 {
            return Err(AppError::invalid(
                absolute_base + i,
                "station name is empty",
            ));
        }

        let semi = i + relative_semi;
        let name = &data[i..semi];
        let temp_offset = semi + 1;
        let (temp, consumed) =
            parse_temperature(&data[temp_offset..end], absolute_base + temp_offset)?;
        upsert(map, name, temp, absolute_base + i)?;
        i = temp_offset + consumed;
    }
    Ok(())
}

#[cfg(any(test, not(unix)))]
fn process_chunk(data: &[u8], start: usize, end: usize) -> Result<Map, AppError> {
    let mut map = Map::with_capacity_and_hasher(INITIAL_CAPACITY, RandomState::default());
    process_slice(&mut map, data, start, end, 0)?;
    Ok(map)
}

#[cfg(unix)]
fn read_at_exact(file: &File, path: &Path, offset: u64, out: &mut [u8]) -> Result<(), AppError> {
    use std::os::unix::fs::FileExt as _;

    file.read_exact_at(out, offset)
        .map_err(|error| io_error("reading", path, error))
}

#[cfg(unix)]
fn aligned_file_bounds(
    file: &File,
    path: &Path,
    len: u64,
    workers: usize,
) -> Result<Vec<(u64, u64)>, AppError> {
    if len == 0 {
        return Ok(vec![(0, 0)]);
    }

    let mut points = Vec::with_capacity(workers + 1);
    points.push(0);
    for index in 1..workers {
        let approximate = ((u128::from(len) * index as u128) / workers as u128) as u64;
        let remaining = usize::try_from((len - approximate).min(MAX_RECORD_LEN as u64))
            .expect("MAX_RECORD_LEN fits in usize");
        let mut probe = [0u8; MAX_RECORD_LEN];
        read_at_exact(file, path, approximate, &mut probe[..remaining])?;
        let Some(newline) = memchr(b'\n', &probe[..remaining]) else {
            return Err(AppError::invalid(
                usize::try_from(approximate).unwrap_or(usize::MAX),
                "record exceeds the maximum supported length",
            ));
        };
        points.push(approximate + newline as u64 + 1);
    }
    points.push(len);
    Ok(points.windows(2).map(|pair| (pair[0], pair[1])).collect())
}

#[cfg(unix)]
fn logical_file_len(file: &File, path: &Path, physical_len: u64) -> Result<u64, AppError> {
    let tail_len = usize::try_from(physical_len.min((MAX_RECORD_LEN + 16) as u64))
        .expect("the bounded tail length fits in usize");
    let tail_start = physical_len - tail_len as u64;
    let mut tail = vec![0u8; tail_len];
    read_at_exact(file, path, tail_start, &mut tail)?;

    let Some(newline) = memchr::memrchr(b'\n', &tail) else {
        return Ok(physical_len);
    };
    let trailing = &tail[newline + 1..];
    if trailing.iter().all(|&byte| byte == 0) {
        Ok(tail_start + newline as u64 + 1)
    } else {
        Ok(physical_len)
    }
}

#[cfg(unix)]
fn process_file_range(file: &File, path: &Path, start: u64, end: u64) -> Result<Map, AppError> {
    let mut map = Map::with_capacity_and_hasher(INITIAL_CAPACITY, RandomState::default());
    if start == end {
        return Ok(map);
    }

    let mut buffer = vec![0u8; BLOCK_SIZE + MAX_RECORD_LEN];
    let mut next_offset = start;
    let mut carry = 0usize;

    while next_offset < end {
        let to_read = usize::try_from((end - next_offset).min(BLOCK_SIZE as u64))
            .expect("BLOCK_SIZE fits in usize");
        let buffer_start = next_offset
            .checked_sub(carry as u64)
            .expect("carry bytes precede the next unread offset");
        read_at_exact(file, path, next_offset, &mut buffer[carry..carry + to_read])?;
        next_offset += to_read as u64;
        let valid = carry + to_read;

        let process_end = if next_offset == end {
            valid
        } else {
            memchr::memrchr(b'\n', &buffer[..valid]).ok_or_else(|| {
                AppError::invalid(
                    usize::try_from(buffer_start).unwrap_or(usize::MAX),
                    "record exceeds the streaming block size",
                )
            })? + 1
        };

        let absolute_base = usize::try_from(buffer_start)
            .map_err(|_| AppError::invalid(0, "file is too large for this target architecture"))?;
        process_slice(
            &mut map,
            &buffer[..process_end],
            0,
            process_end,
            absolute_base,
        )?;

        carry = valid - process_end;
        buffer.copy_within(process_end..valid, 0);
    }
    Ok(map)
}

fn merge_into(acc: &mut Map, map: Map) -> Result<(), AppError> {
    for (name, stats) in map {
        let hash = hash_name(acc.hasher(), &name);
        let at_station_limit = map_len_at_limit(acc.len());
        match acc
            .raw_entry_mut()
            .from_hash(hash, |stored| stored.as_ref() == name.as_ref())
        {
            RawEntryMut::Occupied(mut entry) => entry.get_mut().merge(&stats),
            RawEntryMut::Vacant(entry) => {
                if at_station_limit {
                    return Err(AppError::TooManyStations);
                }
                entry.insert_hashed_nocheck(hash, name, stats);
            }
        }
    }
    Ok(())
}

#[cfg(any(test, not(unix)))]
fn solve(data: &[u8]) -> Result<String, AppError> {
    if data.is_empty() {
        return Ok("{}".to_owned());
    }

    let available = thread::available_parallelism().map_or(1, |n| n.get());
    let useful_workers = data.len().div_ceil(BLOCK_SIZE).max(1);
    let workers = available.min(useful_workers);
    let bounds = chunk_bounds(data, workers);
    let partials: Vec<Result<Map, AppError>> = thread::scope(|scope| {
        let mut handles = Vec::with_capacity(bounds.len());
        for &(start, end) in &bounds {
            handles.push(
                thread::Builder::new()
                    .spawn_scoped(scope, move || process_chunk(data, start, end))
                    .map_err(AppError::ThreadSpawn)?,
            );
        }
        Ok::<_, AppError>(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap_or(Err(AppError::WorkerPanicked)))
                .collect(),
        )
    })?;

    let mut merged = Map::with_capacity_and_hasher(INITIAL_CAPACITY, RandomState::default());
    for partial in partials {
        merge_into(&mut merged, partial?)?;
    }
    let sorted: BTreeMap<Box<[u8]>, Stats> = merged.into_iter().collect();
    Ok(format_output(&sorted))
}

#[cfg(unix)]
fn solve_file(path: &Path) -> Result<String, AppError> {
    let file = File::open(path).map_err(|e| io_error("opening", path, e))?;
    let before = file
        .metadata()
        .map_err(|e| io_error("reading metadata for", path, e))?
        .len();
    usize::try_from(before)
        .map_err(|_| AppError::invalid(0, "file is too large for this target architecture"))?;
    if before == 0 {
        return Ok("{}".to_owned());
    }

    let logical_len = logical_file_len(&file, path, before)?;

    let available = thread::available_parallelism().map_or(1, |n| n.get());
    let useful_workers = usize::try_from(logical_len.div_ceil(BLOCK_SIZE as u64))
        .unwrap_or(available)
        .max(1);
    let workers = available.min(useful_workers);
    let bounds = aligned_file_bounds(&file, path, logical_len, workers)?;
    let partials: Vec<Result<Map, AppError>> = thread::scope(|scope| {
        let file = &file;
        let mut handles = Vec::with_capacity(bounds.len());
        for &(start, end) in &bounds {
            handles.push(
                thread::Builder::new()
                    .spawn_scoped(scope, move || process_file_range(file, path, start, end))
                    .map_err(AppError::ThreadSpawn)?,
            );
        }
        Ok::<_, AppError>(
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap_or(Err(AppError::WorkerPanicked)))
                .collect(),
        )
    })?;

    let after = file
        .metadata()
        .map_err(|e| io_error("re-reading metadata for", path, e))?
        .len();
    if before != after {
        return Err(AppError::InputChanged {
            path: path.to_path_buf(),
        });
    }

    let mut merged = Map::with_capacity_and_hasher(INITIAL_CAPACITY, RandomState::default());
    for partial in partials {
        merge_into(&mut merged, partial?)?;
    }
    let sorted: BTreeMap<Box<[u8]>, Stats> = merged.into_iter().collect();
    Ok(format_output(&sorted))
}

#[cfg(not(unix))]
fn solve_file(path: &Path) -> Result<String, AppError> {
    let data = read_input(path)?;
    solve(&data)
}

fn parse_path() -> Result<PathBuf, AppError> {
    let mut args = std::env::args_os().skip(1);
    let Some(path) = args.next() else {
        return Err(AppError::Usage);
    };
    if args.next().is_some() {
        return Err(AppError::Usage);
    }
    Ok(PathBuf::from(path))
}

fn run() -> Result<(), AppError> {
    let path = parse_path()?;
    let output = solve_file(&path)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{output}").map_err(|e| io_error("writing stdout for", &path, e))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "error: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_valid_temperature() {
        for value in -999i32..=999 {
            let abs = value.unsigned_abs();
            let text = if value < 0 {
                format!("-{}.{:01}\n", abs / 10, abs % 10)
            } else {
                format!("{}.{:01}\n", abs / 10, abs % 10)
            };
            assert_eq!(
                parse_temperature(text.as_bytes(), 0).unwrap(),
                (value, text.len())
            );
        }
    }

    #[test]
    fn rejects_malformed_temperatures() {
        for bad in [
            b"".as_slice(),
            b"1.0",
            b"1\n",
            b"1.00\n",
            b"100.0\n",
            b"+1.0\n",
            b"a.0\n",
            b"1.a\n",
            b"1,0\n",
        ] {
            assert!(parse_temperature(bad, 7).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn solves_without_tail_padding() {
        let output = solve(b"Rome;1.0\nRome;-2.0\nOslo;0.5\n").unwrap();
        assert_eq!(output, "{Oslo=0.5/0.5/0.5, Rome=-2.0/-0.5/1.0}");
    }

    #[test]
    fn accepts_a_hundred_byte_station_name() {
        let name = "x".repeat(MAX_NAME_LEN);
        let input = format!("{name};1.0\n");
        let output = solve(input.as_bytes()).unwrap();
        assert!(output.starts_with(&format!("{{{name}=")));
    }

    #[test]
    fn rejects_invalid_names_and_records() {
        assert!(solve(b";1.0\n").is_err());
        assert!(solve(format!("{};1.0\n", "x".repeat(101)).as_bytes()).is_err());
        assert!(solve(b"Rome 1.0\n").is_err());
        assert!(solve(b"Rome;1.0").is_err());
        assert!(solve(b"\xff;1.0\n").is_err());
    }

    #[test]
    fn enforces_the_official_station_limit() {
        let mut input = String::new();
        for station in 0..=MAX_STATIONS {
            use std::fmt::Write as _;
            writeln!(input, "S{station:05};1.0").unwrap();
        }
        assert!(matches!(
            solve(input.as_bytes()),
            Err(AppError::TooManyStations)
        ));
    }
}
