//! `std::io` - standard input.
//!
//! Part III 17.1 and [ADR-019](../../../docs/specification/adr/adr-019.md). A
//! stream is not a file, and the surface says so: no map, no seek, no length,
//! and no second read of the same bytes. `std::fs`'s zero-copy story rests on
//! pages that exist before the program asks for them; stdin's bytes do not
//! exist until they are read, so a program that wants views into its input owns
//! the buffer first.
//!
//! Neither function is `sync`: a read from a pipe is a suspension point, which
//! is exactly what the `sync` rule (Part II, 12.1) exists to keep out of a
//! `par_iter` body.
//!
//! **Whole-of-input reads suspend; a line at a time does not yet**
//! ([ADR-121](../../../docs/specification/adr/adr-121.md) D4).
//!
//! Since [ADR-055](../../../docs/specification/adr/adr-055.md) §6 step 3 these
//! signatures have said a read from here may pause, which is what a caller's
//! compiler reads and what makes the enclosing function `async`. The bodies did
//! not: the blocking read ran on the program's own thread, because a worker's
//! reply could not wake an executor parked on the ring and a future fed from one
//! had nowhere to come back to.
//!
//! ADR-121 D1 put the bell on the ring, so [`read`] and [`read_to_string`] are
//! now the read they always were, performed on an I/O worker and **awaited**.
//! There is still no second read shape and no ring path for a stream with no
//! size to `stat`, which is D4's own sentence: what changed is the park.
//!
//! **And [`lines`] has moved too**
//! ([ADR-172](../../../docs/specification/adr/adr-172.md) D4), which took a
//! decision about the *language* rather than a change to the runtime. A step of
//! it was an `Iterator::next`, and a suspension point inside one is
//! `while let Some(x) = s.next().await` below - so D1 decided that a `for` may
//! iterate something whose step pauses and writes that form, and [`Lines`] has
//! an inherent `async fn next` for it to write. The chunking is what keeps it
//! from costing more than it saves: a hop to a worker per *line* would be
//! slower than the blocking reader, so a buffer comes back at a time.

/// What a `std` call fails with ([ADR-280](../../../docs/specification/adr/adr-280.md) D5).
///
/// Part I 7.1's error type, for the library: an `enum` with payload, `impl
/// Error`, and variants a `catch` can tell apart. Until this existed every
/// `std` entry wrote `throws = ["?"]` — *something this compiler cannot name* —
/// which is the absence of an answer standing in for one, and it made
/// [ADR-023](../../../docs/specification/adr/adr-023.md) D1's set unusable for
/// every program that reads a file.
///
/// **It lives in `io` and is written out.** `use std::io` and
/// `io::IoError::NotFound(…)` is what a program that matches on it writes, which
/// is [ADR-313](../../../docs/specification/adr/adr-313.md) D3's rule applied
/// straight: what needs no prefix is Part I 1.3's list, and this is not on it.
/// `fs::read` throwing an `io` type is the cost, accepted: a program that only
/// *propagates* the failure names nothing, and one that takes it apart writes
/// the second `use`.
///
/// **Why these four.** D4 of [ADR-023](../../../docs/specification/adr/adr-023.md)
/// makes the variants of one error type **closed**, so each is a lasting
/// commitment and the list is short on purpose:
///
/// * `NotFound` is the specification's own — Appendix A.1 calls *a missing
///   file* the example of a recoverable error, and D6's worked output writes
///   `IoError::NotFound`;
/// * `PermissionDenied` is the other environmental failure a program acts on
///   **differently** rather than reports;
/// * `NotText` is `std`'s own rather than the operating system's: three places
///   here check UTF-8 and report `InvalidData`, and Part III 17.1 specifies the
///   check;
/// * `Other` is D4's **named residue** — *exhaustiveness degrades to "and
///   anything else"* — carrying what the operating system said.
///
/// **The payload is owned text and not a view.**
/// [ADR-023](../../../docs/specification/adr/adr-023.md) D10 says a path that
/// came from a buffer should travel as a view; that is the tether
/// ([ADR-tether-v1](../../../docs/specification/adr/adr-tether-v1.md)), which is not built,
/// and a lifetime on this type would reach every `std` signature. One
/// allocation on a path that is already failing is the honest price until then.
/// What an [`IoError`] from this module was about: a stream has no path, and
/// the name is what a message can say instead.
const STDIN: &str = "standard input";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoError {
    /// It is not there. The payload is what was looked for, a file's name in
    /// the platform's own bytes (ADR-319 D7).
    NotFound(crate::fs::Path),
    /// It is there and this program may not have it.
    PermissionDenied(crate::fs::Path),
    /// The bytes are not text, and text in this language is UTF-8.
    NotText(String),
    /// **The name would leave the root it was given**
    /// ([ADR-108](../../../docs/specification/adr/adr-108.md) D3), and this is
    /// a refusal rather than a rewritten name: a program that quietly serves a
    /// *different* file than the one asked for is a worse bug than one that
    /// serves nothing.
    ///
    /// The payload is what was asked for, as it was asked for — the resolved
    /// name is not in it, because a message that printed where the name landed
    /// would tell whoever sent it what is on the machine.
    ///
    /// A case beside `NotFound` and not an error type of its own, which is what
    /// D3's *one case beside not found among the errors the entry already names*
    /// asks for: every path-taking entry already `throws = ["io::IoError"]`, so
    /// the root check adds no member to any program's failure set.
    Outside(crate::fs::Path),
    /// Everything else, as the operating system said it.
    Other(String),
}

impl IoError {
    /// What went wrong, in words: `Error`'s `message` (Part I 7.1), which is
    /// what the failure prints as. A block's text (#490), the `String` of a
    /// program.
    pub fn message(&self) -> crate::block::Text {
        crate::block::Text::from(self.to_string())
    }

    /// One of the language below's failures, told apart and given what it was
    /// about.
    ///
    /// `what` is the path, or the name of the stream: an operating system's
    /// error does not carry it, and *not found* without the thing that was not
    /// found is the message D3 calls a round trip to the user.
    ///
    /// A name keeps its bytes where the variant carries one; a message shows
    /// it (ADR-319 D6, D7).
    pub fn of(error: std::io::Error, what: impl AsRef<std::ffi::OsStr>) -> IoError {
        let what = std::path::Path::new(what.as_ref());
        match error.kind() {
            std::io::ErrorKind::NotFound => IoError::NotFound(what.to_path_buf()),
            std::io::ErrorKind::PermissionDenied => IoError::PermissionDenied(what.to_path_buf()),
            std::io::ErrorKind::InvalidData => IoError::NotText(what.display().to_string()),
            _ => IoError::Other(format!("{}: {error}", what.display())),
        }
    }

    /// What was looked for, shown as text.
    pub fn what(&self) -> std::borrow::Cow<'_, str> {
        match self {
            IoError::NotFound(name) | IoError::PermissionDenied(name) | IoError::Outside(name) => {
                name.to_string_lossy()
            }
            IoError::NotText(what) | IoError::Other(what) => what.into(),
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::NotFound(name) => {
                write!(f, "no such file or directory: {}", name.display())
            }
            IoError::PermissionDenied(name) => write!(f, "permission denied: {}", name.display()),
            IoError::NotText(what) => write!(f, "not valid UTF-8: {what}"),
            IoError::Outside(name) => {
                write!(
                    f,
                    "the name leaves the root it was given: {}",
                    name.display()
                )
            }
            IoError::Other(what) => f.write_str(what),
        }
    }
}

impl std::error::Error for IoError {}

/// **`println`, `print`, `eprintln` and `eprint`**
/// ([ADR-392](../../../docs/specification/adr/adr-392.md)): a failed write to
/// standard output or standard error is an [`IoError`], never a panic (Rust's
/// `println!` panics on a closed pipe). The program's text arrives as
/// `format_args!`, which the emitter writes from the call's argument.
pub fn println(args: std::fmt::Arguments<'_>) -> Result<(), IoError> {
    written(std::io::stdout().lock(), args, true, STDOUT)
}

/// [`println`] without the line end.
pub fn print(args: std::fmt::Arguments<'_>) -> Result<(), IoError> {
    written(std::io::stdout().lock(), args, false, STDOUT)
}

/// [`println`] to standard error.
pub fn eprintln(args: std::fmt::Arguments<'_>) -> Result<(), IoError> {
    written(std::io::stderr().lock(), args, true, STDERR)
}

/// [`print`] to standard error.
pub fn eprint(args: std::fmt::Arguments<'_>) -> Result<(), IoError> {
    written(std::io::stderr().lock(), args, false, STDERR)
}

const STDOUT: &str = "standard output";
const STDERR: &str = "standard error";

/// One print: the text, then the line end where asked, under one lock, so two
/// tasks' lines do not interleave.
fn written(
    mut to: impl std::io::Write,
    args: std::fmt::Arguments<'_>,
    newline: bool,
    what: &str,
) -> Result<(), IoError> {
    let wrote = match newline {
        true => to.write_fmt(format_args!("{args}\n")),
        false => to.write_fmt(args),
    };
    wrote.map_err(|error| print_error(error, what))
}

/// **Bytes printed as they are** (ADR-019 D7, ADR-319 D4): `print(b)` of
/// `Bytes` and a file's name, to standard output or standard error, through the
/// buffer text goes through, with a newline where the call says so, under one
/// lock.
pub fn print_bytes(bytes: &[u8], to_err: bool, newline: bool) -> Result<(), IoError> {
    match to_err {
        true => bytes_written(std::io::stderr().lock(), bytes, newline, STDERR),
        false => bytes_written(std::io::stdout().lock(), bytes, newline, STDOUT),
    }
}

fn bytes_written(
    mut to: impl std::io::Write,
    bytes: &[u8],
    newline: bool,
    what: &str,
) -> Result<(), IoError> {
    let mut wrote = to.write_all(bytes);
    if newline && wrote.is_ok() {
        wrote = to.write_all(b"\n");
    }
    wrote.map_err(|error| print_error(error, what))
}

/// A failed write's error: the stream's name and what the operating system
/// said, `standard output: Broken pipe (os error 32)`.
pub(crate) fn print_error(error: std::io::Error, what: &str) -> IoError {
    IoError::Other(format!("{what}: {error}"))
}

/// All of standard input, as text.
///
/// UTF-8 validated, for the reason `fs::read_to_string` validates: a parser
/// handed bytes that are not text would find that out one view at a time, and a
/// frame boundary is a string.
///
/// Reading it a second time yields what the operating system says, which is
/// nothing.
pub async fn read_to_string() -> Result<crate::block::Text, IoError> {
    let bytes = crate::rt::io::stdin_whole()
        .await
        .map_err(|e| IoError::of(e, STDIN))?;
    // The same failure `std`'s own `read_to_string` reports, under the name
    // this library gives it: a stream that is not text is `NotText` and not a
    // lossy string. The checked text is a block's, the same memory (#490).
    String::from_utf8(bytes)
        .map(crate::block::Text::from)
        .map_err(|_| IoError::NotText(STDIN.to_string()))
}

/// All of standard input, as bytes.
///
/// The half for input that is not text - and the one that says what
/// `read_to_string` is doing, since the only difference is the check.
///
/// A program's `Vec[u8]` is a block's list ([`crate::block::List`]), so the
/// bytes are handed over as one, the same memory: a plain `Vec` here was a
/// rustc error wherever the result met a `Vec[u8]` the program declared
/// (#1619).
pub async fn read() -> Result<crate::block::List<u8>, IoError> {
    crate::rt::io::stdin_whole()
        .await
        .map(crate::block::List::from)
        .map_err(|e| IoError::of(e, STDIN))
}

/// Standard input, one line at a time.
///
/// Part III 17.1, and the first thing here that reads an unbounded input in
/// **constant memory**: nothing holds the whole stream, so a pipe that never
/// ends is a program that never grows.
///
/// The lines are **owned**, and 17.1 says why: a file's line can be a view
/// because the file is still there to point at, and a stream's bytes are gone
/// once consumed. Keeping them would be `read_to_string` with extra steps.
/// That is also what keeps this expressible at all - an iterator may hand out
/// views into a buffer it does not own, and never into one it does
/// ([ADR-loop-failure-v1](../../../docs/specification/adr/adr-loop-failure-v1.md), Wall B).
///
/// **A step can fail**, and the item says so rather than pretending a failed
/// read is the end of the stream - which would turn a truncated input into a
/// shorter one, silently, and is the bug class Part I 6.4 refuses by name. What
/// a Nikaia program writes has none of that in it:
///
/// ```nika
/// fn count() -> i64 throws {
///     let mut n = 0
///     for line in io::lines() { n += 1 }
///     return n
/// }
/// ```
///
/// The failure travels out of the loop and out of the function, and the
/// compiler is what made `throws` be there (`NK2701`). `std.contracts` records
/// `iterates = "throws"` on `Lines`, which is how a compiler that cannot see
/// this body knows.
///
/// The trailing newline is not part of a line, and a final line without one is
/// still a line.
///
/// **This one does not suspend**, and the module comment says why: a step is an
/// `Iterator::next`, and a suspension point inside one needs a `for` over a
/// stream that neither this language nor the one below has decided.
/// [`read_to_string`] is the entry that does, for an input a program is willing
/// to hold whole.
pub async fn lines() -> Lines {
    Lines {
        _shared: (),
        pool: None,
    }
}

/// One line of standard input, without its line ending; `null` at the end of
/// the input (Part III 17.1).
///
/// **It reads from the buffer [`lines`] reads from**, so the two can be mixed:
/// a header read with this and a body read with [`read_exact`] leave the next
/// line where `lines()` finds it. A final line without a newline is still a
/// line, as in `lines()`.
///
/// It pauses while the stream has nothing to give, and fails where the line is
/// not UTF-8 or the input cannot be read.
pub async fn read_line() -> Result<Option<crate::block::Text>, IoError> {
    loop {
        if let Some(line) = with_held(Held::take_a_line) {
            return line.map(Some);
        }
        if with_held(|held| held.ended) {
            return Ok(None);
        }
        refill().await?;
    }
}

/// Exactly `n` bytes of standard input (Part III 17.1).
///
/// The framing of a protocol that says how long its body is - a
/// `Content-Length` header - needs exactly this: the body ends without a
/// newline, so no line reader can find its end. It reads from the buffer
/// [`lines`] and [`read_line`] read from.
///
/// **A short read is an error, not a shorter result** (Part I 6.4): an input
/// that ends before `n` bytes have come is `IoError`, and what did come is
/// not handed out as if it were the whole.
pub async fn read_exact(n: i64) -> Result<crate::bytes::Bytes, IoError> {
    let want = usize::try_from(n).map_err(|_| {
        IoError::of(
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("read_exact of {n} bytes"),
            ),
            STDIN,
        )
    })?;
    loop {
        if let Some(bytes) = with_held(|held| held.take_exact(want)) {
            return Ok(bytes.into());
        }
        if with_held(|held| held.ended) {
            return Err(IoError::of(
                std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    format!("the input ended before {want} bytes"),
                ),
                STDIN,
            ));
        }
        refill().await?;
    }
}

/// **The one buffer of standard input** a process has: what the last chunks
/// brought and what has not been handed out yet. [`lines`], [`read_line`] and
/// [`read_exact`] all take from it, which is what lets them be mixed (#1347).
static HELD: std::sync::Mutex<Held> = std::sync::Mutex::new(Held {
    held: Vec::new(),
    at: 0,
    ended: false,
});

/// `step` on the process's buffer. The lock is never held across an `await`.
fn with_held<T>(step: impl FnOnce(&mut Held) -> T) -> T {
    let mut held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    step(&mut held)
}

/// One more chunk of standard input into the buffer, or its end.
///
/// **Standard output is flushed first**, as C's stdio does before it reads a
/// line-buffered input: what a program wrote before it waits for an answer -
/// a prompt without its newline, a framed response whose body ends without one
/// (#1347) - reaches the other side before the program waits for it.
async fn refill() -> Result<(), IoError> {
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    match crate::rt::io::stdin_chunk(CHUNK).await {
        Err(e) => {
            // The stream is over either way: a reader that kept asking
            // after a failure would loop on it.
            with_held(|held| held.ended = true);
            Err(IoError::of(e, STDIN))
        }
        Ok(chunk) => {
            with_held(|held| held.append(&chunk));
            Ok(())
        }
    }
}

/// What [`lines`] hands back.
///
/// A named type rather than `impl Iterator`, because the ledger names types and
/// a caller's compiler looks this one up by name (ADR-020).
///
/// **Its step pauses** ([ADR-172](../../../docs/specification/adr/adr-172.md)
/// D4), and the buffer is why that costs nothing: a hop to an I/O worker and
/// back is a wake-up, and paying one per *line* would make this slower than
/// the blocking reader it replaces. So a chunk comes back at a time and the
/// line endings are found here — one hop per buffer, and a program that reads
/// a million lines pays as many hops as it has buffers.
///
/// **Not an `Iterator`**, and that is the decision rather than an omission.
/// `Iterator::next` has no `await` in it, which is the whole reason D1 exists;
/// the inherent `async fn next` below is the shape a `for` over a pausing
/// sequence lowers to. The trait a second such producer wanted is
/// `nikaia_std::seq::Step` ([ADR-233](../../../docs/specification/adr/adr-233.md)
/// D1), and this is one: its item is the line or the failure.
pub struct Lines {
    /// The buffer is the process's ([`HELD`]), shared with [`read_line`] and
    /// [`read_exact`]; a `Lines` is only the handle a `for` walks.
    _shared: (),
    /// **The pool a `for` over the lines reads into** ([`Lines::in_pool`]),
    /// where its lines are views; none where the lines are walked as values.
    pool: Option<Pool>,
}

/// **A fixed pool of stream buffers** (ADR-stream-buffers-v1 D1, D3, D5): at
/// most `size` chunks of standard input, each in a keep of its own, which the
/// lines of a `for` view. A chunk no line is held in any more (its keep's
/// count is down to the pool's own) is the next read's buffer, so a loop in
/// its steady state allocates nothing, per chunk or per line.
struct Pool {
    /// The size the compiler counted: the names that hold a line across turns,
    /// the chunk being read and the one a cut line moves into (D3).
    size: usize,
    chunks: Vec<Chunk>,
    /// The chunk the lines are cut from now.
    current: usize,
    /// How far into it the lines already handed out reach.
    at: usize,
    /// The stream is over: what is left after `at` is the last line.
    ended: bool,
    /// More chunks are held than the compiler counted, under `nikaia test`:
    /// the next line stops the run at its loop (D9, ADR-tether-v1 D15).
    over: bool,
    /// The bytes of the line handed out last, in the current chunk, which
    /// [`Lines::again`] hands out once more.
    last: (usize, usize),
    /// The reply channel of every read of this loop, kept so a read allocates
    /// none.
    reply: std::sync::mpsc::SyncSender<(Vec<u8>, std::io::Result<usize>)>,
    answer: std::sync::mpsc::Receiver<(Vec<u8>, std::io::Result<usize>)>,
}

/// One buffer of a [`Pool`]: its keep, which the lines' holders count, and
/// its bytes, shared with that keep while the keep holds them.
struct Chunk {
    keep: std::sync::Arc<crate::tether::Keep>,
    bytes: std::sync::Arc<Vec<u8>>,
    place: crate::tether::Place,
}

/// A chunk's bytes as the keep holds them: the other handle on them is the
/// pool's, which changes them only once this one is dropped
/// ([`std::sync::Arc::get_mut`]), so they stay where they are while a view of
/// them lives.
struct ChunkBytes(std::sync::Arc<Vec<u8>>);

impl crate::tether::Viewed for ChunkBytes {
    fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// **One line of a `for` over [`Lines::in_pool`]**: a view of the chunk it was
/// cut from, the keep that chunk lives in and its place there, which a name
/// that holds the line across turns takes a count on
/// (`tether::Held::in_place`, ADR-stream-buffers-v1 D2).
pub struct Line<'a> {
    text: &'a str,
    keep: &'a std::sync::Arc<crate::tether::Keep>,
    place: crate::tether::Place,
    /// The pool ran over under `nikaia test` (D9).
    over: Option<usize>,
}

impl<'a> Line<'a> {
    /// The text, its keep and its place, as the loop binds them.
    ///
    /// **More buffers held than the compiler counted is the compiler's
    /// fault** (ADR-stream-buffers-v1 D3, D9; ADR-tether-v1 D15): a program
    /// takes one more buffer and goes on, and a run under `nikaia test`
    /// panics here, at the loop's line.
    #[track_caller]
    #[inline]
    pub fn parts(
        self,
    ) -> (
        &'a str,
        &'a std::sync::Arc<crate::tether::Keep>,
        crate::tether::Place,
    ) {
        if let Some(size) = self.over {
            over_the_pool(size);
        }
        (self.text, self.keep, self.place)
    }
}

/// More chunks of a stream held at once than the pool the compiler counted.
#[track_caller]
#[cold]
fn over_the_pool(size: usize) -> ! {
    panic!(
        "a loop over a stream holds more than the {size} buffers the compiler counted for it (ADR-stream-buffers-v1 D3, D9). This is a bug in Nikaia, not in your program: the compiler's fault in counting where its lines are kept. Please report it, with this message, at https://github.com/Nikaia-Foundation/Nikaia/issues."
    )
}

/// Whether this run is one of `nikaia test`'s, which names a directory for
/// every test it runs (ADR-247 D5). Read once.
fn under_test() -> bool {
    static UNDER_TEST: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *UNDER_TEST.get_or_init(|| std::env::var_os(crate::fs::TEST_DIR_VAR).is_some())
}

/// What standard input has brought and not yet handed out.
struct Held {
    /// What the last chunks brought and what has not been handed out yet.
    held: Vec<u8>,
    /// How far into `held` the lines already handed out reach.
    at: usize,
    /// The stream is over: no further chunk is asked for, and what is left in
    /// `held` is the last line if it is not empty.
    ended: bool,
}

/// How much of standard input one hop to a worker asks for.
///
/// **A pipe's capacity on Linux**, so that one read can take everything a full
/// pipe holds (#423). It was `BufReader`'s 8 KiB, and on a pipe that is already
/// full - `cat file | prog` - the hops were the cost: 4 M lines (60 MB) took
/// 791 ms at 8 KiB and 414 ms at 64 KiB, best of 9, and 671 against 389 ms
/// read from a file redirected to standard input. It is still **one read** a
/// hop, so a slow pipe hands over whatever is there, as before. Zeroing the
/// buffer before the read, measured on its own, cost nothing that showed.
const CHUNK: usize = 64 * 1024;

impl Lines {
    /// **The lines read into a pool of `size` buffers and handed out as
    /// views** (ADR-stream-buffers-v1 D1, D3): what a `for` over the lines is
    /// lowered to, `size` the count `--tethers` names for the loop. What
    /// standard input has brought and not handed out yet is the first
    /// buffer's start, and what the loop leaves unread goes back there when
    /// it ends, so [`read_line`] after it finds the next line.
    pub fn in_pool(mut self, size: i64) -> Lines {
        let size = usize::try_from(size).unwrap_or(2).max(2);
        let (held, ended) = with_held(|held| {
            let rest = held.held.split_off(held.at);
            held.held.clear();
            held.at = 0;
            (rest, held.ended)
        });
        let (reply, answer) = std::sync::mpsc::sync_channel(1);
        let mut pool = Pool {
            size,
            chunks: Vec::with_capacity(size),
            current: 0,
            at: 0,
            ended,
            over: false,
            last: (0, 0),
            reply,
            answer,
        };
        let mut first = Pool::buffer();
        first.extend_from_slice(&held);
        pool.chunks.push(Pool::chunk(first));
        self.pool = Some(pool);
        self
    }

    /// **The next line of the pool, as a view** of the chunk it was cut from
    /// ([`Lines::in_pool`]), pausing while the stream has nothing to give.
    /// A line that is not UTF-8 is the step's [`IoError`], as in
    /// [`Lines::next`]; the line endings are the same.
    pub async fn next_line(&mut self) -> Option<Result<Line<'_>, IoError>> {
        let pool = self.pool.as_mut()?;
        let (from, to) = loop {
            if let Some(found) = pool.cut() {
                break found;
            }
            if pool.ended {
                return None;
            }
            if let Err(e) = pool.refill().await {
                return Some(Err(e));
            }
        };
        let over = if std::mem::take(&mut pool.over) {
            Some(pool.size)
        } else {
            None
        };
        pool.last = (from, to);
        let chunk = &pool.chunks[pool.current];
        match std::str::from_utf8(&chunk.bytes[from..to]) {
            Ok(text) => Some(Ok(Line {
                text,
                keep: &chunk.keep,
                place: chunk.place,
                over,
            })),
            Err(e) => Some(Err(IoError::of(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                STDIN,
            ))),
        }
    }

    /// The next line, **pausing** while the stream has nothing to give
    /// ([ADR-172](../../../docs/specification/adr/adr-172.md) D4).
    ///
    /// **`IoError` and not the language below's own**
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D5): a step of
    /// this fails, so the function around the `for` says `throws` — and what it
    /// throws is what `std` says it throws, here as everywhere else. This was
    /// the one surface that record missed, and it showed the day a program
    /// whose whole set was `io::IoError` got a **named** channel
    /// ([ADR-280](../../../docs/specification/adr/adr-280.md) D9): the `?` the
    /// loop's step takes had a raw `std::io::Error` on the left of it and a
    /// named one on the right.
    ///
    /// The trailing newline is not part of a line, a `\r` before it is not
    /// either, and a final line without one is still a line — which is
    /// `BufRead::lines`' own rule, kept because programs were written against
    /// it.
    pub async fn next(&mut self) -> Option<Result<crate::block::Text, IoError>> {
        loop {
            if let Some(line) = with_held(Held::take_a_line) {
                return Some(line);
            }
            if with_held(|held| held.ended) {
                return None;
            }
            if let Err(e) = refill().await {
                return Some(Err(e));
            }
        }
    }

    /// Everything the sequence produces, as a list — `Seq::collect` over a
    /// sequence whose step pauses
    /// ([ADR-172](../../../docs/specification/adr/adr-172.md) D5).
    ///
    /// **The eager walks**: what `collect` and `nth` hand back is a value of
    /// its own, so each is a loop around [`Lines::next`] and nothing more;
    /// `count`, `join`, `fold`, `any` and `all` walk the lines as views of a
    /// pool ([`Views`]). The lazy ones hand back another sequence whose step
    /// pauses and can fail, `nikaia_std::seq::Failing`
    /// ([ADR-234](../../../docs/specification/adr/adr-234.md)), or a
    /// [`Views`].
    ///
    /// **The first failure ends the walk.** A `collect` that swallowed one
    /// would turn a truncated stream into a shorter list, which is the bug
    /// class Part I 6.4 refuses by name — and is what this entry did until
    /// D5, when `Lines` was an `Iterator` over `Result` and the ledger said
    /// the result was a list of strings.
    pub async fn collect(mut self) -> Result<crate::block::List<crate::block::Text>, IoError> {
        let mut out = crate::block::List::new();
        while let Some(line) = self.next().await {
            out.push(line?);
        }
        Ok(out)
    }

    /// How many lines the sequence produces — `Seq::count`, each line a view
    /// of the pool and none copied (ADR-stream-buffers-v1 D1, [`Views`]).
    pub async fn count(self) -> Result<i64, IoError> {
        Views::of(self).count().await
    }

    /// The line at a position, or nothing where the stream is shorter than that
    /// — `Seq::nth`. It walks up to that position to answer.
    pub async fn nth(mut self, at: i64) -> Result<Option<crate::block::Text>, IoError> {
        if at < 0 {
            return Ok(None);
        }
        let mut seen = 0_i64;
        while let Some(line) = self.next().await {
            let line = line?;
            if seen == at {
                return Ok(Some(line));
            }
            seen += 1;
        }
        Ok(None)
    }

    /// Every line written one after another with this text between them —
    /// `Seq::join`, each line a view of the pool ([`Views`]).
    pub async fn join(self, separator: &str) -> Result<crate::block::Text, IoError> {
        Views::of(self).join(separator).await
    }

    /// Each line through a lambda — `Seq::map`, lazy: nothing is read until
    /// something walks the result
    /// ([ADR-234](../../../docs/specification/adr/adr-234.md) D1). The lambda
    /// sees each line as a view of the pool (ADR-stream-buffers-v1 D1,
    /// [`Views`]). A failure passes through to that walk, which is where the
    /// program's `?` is.
    pub fn map<U, F: FnMut(&str) -> U>(self, f: F) -> crate::seq::Failing<ViewMap<Every, F>> {
        Views::of(self).map(f)
    }

    /// Only the lines a lambda says yes to — `Seq::filter`, lazy, each a
    /// view of the pool ([`Views`]).
    pub fn filter<F: FnMut(&&str) -> bool>(self, f: F) -> Views<ViewFilter<Every, F>> {
        Views::of(self).filter(f)
    }

    /// A copy of each line — `Seq::cloned`, the copy the program asks for
    /// (ADR-stream-buffers-v1 D10): the general walk, which hands each line
    /// over as text of its own.
    pub fn cloned(self) -> crate::seq::Failing<Lines> {
        crate::seq::Failing(self)
    }

    /// The lines folded into one value — `Seq::fold`, each a view of the
    /// pool ([`Views`]).
    pub async fn fold<A, F: FnMut(A, &str) -> A>(self, init: A, f: F) -> Result<A, IoError> {
        Views::of(self).fold(init, f).await
    }

    /// Whether a lambda says yes to some line — `Seq::any`, stopping at the
    /// first ([`Views`]).
    pub async fn any<F: FnMut(&str) -> bool>(self, f: F) -> Result<bool, IoError> {
        Views::of(self).any(f).await
    }

    /// Whether a lambda says yes to every line — `Seq::all`, stopping at the
    /// first no ([`Views`]).
    pub async fn all<F: FnMut(&str) -> bool>(self, f: F) -> Result<bool, IoError> {
        Views::of(self).all(f).await
    }

    /// Each line through a lambda that pauses
    /// ([ADR-233](../../../docs/specification/adr/adr-233.md) D1), which
    /// `map` is lowered to there: the lambda sees the line in its buffer,
    /// which only the walk reads, so it stays there across the pause
    /// ([`Views`]).
    pub fn then<U, F: AsyncFnMut(&str) -> U>(
        self,
        f: F,
    ) -> crate::seq::Failing<ViewThen<Every, F>> {
        Views::of(self).then(f)
    }

    /// Only the lines a lambda that pauses says yes to, each a view
    /// ([`Views`]).
    pub fn then_filter<F: AsyncFnMut(&&str) -> bool>(
        self,
        f: F,
    ) -> Views<ViewThenFilter<Every, F>> {
        Views::of(self).then_filter(f)
    }

    /// The first `n` lines — `Seq::take`, lazy, each a view ([`Views`]).
    pub fn take(self, n: usize) -> Views<ViewTake<Every>> {
        Views::of(self).take(n)
    }

    /// Everything after the first `n` lines — `Seq::skip`, lazy ([`Views`]).
    pub fn skip(self, n: usize) -> Views<ViewSkip<Every>> {
        Views::of(self).skip(n)
    }

    /// The first line, and then every `step`-th after it — `Seq::step_by`
    /// ([`Views`]).
    pub fn step_by(self, step: usize) -> Views<ViewStepBy<Every>> {
        Views::of(self).step_by(step)
    }

    /// Pairs, one line and one item of `other` — `Seq::zip`, lazy.
    pub fn zip<J: IntoIterator>(
        self,
        other: J,
    ) -> crate::seq::Failing<crate::seq::OkZip<Self, J::IntoIter>> {
        crate::seq::Failing(self).zip(other)
    }

    /// The line [`Lines::next_line`] handed out last, once more, as a view:
    /// what a walk that decided on it hands on ([`Views::next_line`]).
    fn again(&self, over: Option<usize>) -> Option<Line<'_>> {
        let pool = self.pool.as_ref()?;
        let chunk = &pool.chunks[pool.current];
        let (from, to) = pool.last;
        let text = std::str::from_utf8(&chunk.bytes[from..to]).ok()?;
        Some(Line {
            text,
            keep: &chunk.keep,
            place: chunk.place,
            over,
        })
    }
}

/// **A walk over the lines as views of the pool** (ADR-stream-buffers-v1 D1,
/// D10): what `filter`, `take`, `skip` and `step_by` on `io::lines()` hand
/// back, and what `map`, `count`, `join`, `fold`, `any` and `all` walk. Each
/// line is cut out of a buffer of [`Lines::in_pool`] and seen by the steps
/// and the lambda for one turn, so no line is copied; `.cloned()` is the copy
/// a program asks for. The steps are `S`, which say of each line whether it
/// goes on and whether the walk is over before the next read.
pub struct Views<S> {
    lines: Lines,
    steps: S,
}

/// **What the steps of a [`Views`] do with each line**: whether it goes on,
/// and whether the walk is over before another line is read, so `take(1)`
/// waits for no second line. A step may pause (`then_filter`): the line
/// stays in its buffer meanwhile, since only the walk reads the pool.
pub trait ViewStep {
    /// Whether `line` goes on through the walk.
    fn pass(&mut self, line: &str) -> impl Future<Output = bool>;
    /// **Whether a line that failed goes on** to the walk's end, which hands
    /// it back: a step that counts lines counts it as one, as the general
    /// walk's `skip`, `take` and `step_by` count a failed step, and `filter`
    /// hands it on (ADR-234 D1).
    fn fail(&mut self) -> bool;
    /// Whether no further line can go on.
    fn over(&self) -> bool;
}

/// Every line goes on: the walk before any step.
pub struct Every;

impl ViewStep for Every {
    async fn pass(&mut self, _line: &str) -> bool {
        true
    }
    fn fail(&mut self) -> bool {
        true
    }
    fn over(&self) -> bool {
        false
    }
}

/// The lines a lambda says yes to — `filter`.
pub struct ViewFilter<S, F> {
    steps: S,
    f: F,
}

impl<S: ViewStep, F: FnMut(&&str) -> bool> ViewStep for ViewFilter<S, F> {
    async fn pass(&mut self, line: &str) -> bool {
        self.steps.pass(line).await && (self.f)(&line)
    }
    fn fail(&mut self) -> bool {
        self.steps.fail()
    }
    fn over(&self) -> bool {
        self.steps.over()
    }
}

/// The lines a lambda that pauses says yes to — `then_filter`, which `filter`
/// is lowered to where its lambda pauses (ADR-233 D1).
pub struct ViewThenFilter<S, F> {
    steps: S,
    f: F,
}

impl<S: ViewStep, F: AsyncFnMut(&&str) -> bool> ViewStep for ViewThenFilter<S, F> {
    async fn pass(&mut self, line: &str) -> bool {
        self.steps.pass(line).await && (self.f)(&line).await
    }
    fn fail(&mut self) -> bool {
        self.steps.fail()
    }
    fn over(&self) -> bool {
        self.steps.over()
    }
}

/// The first `left` lines — `take`.
pub struct ViewTake<S> {
    steps: S,
    left: usize,
}

impl<S> ViewTake<S> {
    fn counted(&mut self) -> bool {
        if self.left == 0 {
            return false;
        }
        self.left -= 1;
        true
    }
}

impl<S: ViewStep> ViewStep for ViewTake<S> {
    async fn pass(&mut self, line: &str) -> bool {
        self.left > 0 && self.steps.pass(line).await && self.counted()
    }
    fn fail(&mut self) -> bool {
        self.left > 0 && self.steps.fail() && self.counted()
    }
    fn over(&self) -> bool {
        self.left == 0 || self.steps.over()
    }
}

/// Everything after the first `left` lines — `skip`.
pub struct ViewSkip<S> {
    steps: S,
    left: usize,
}

impl<S> ViewSkip<S> {
    fn counted(&mut self) -> bool {
        if self.left > 0 {
            self.left -= 1;
            return false;
        }
        true
    }
}

impl<S: ViewStep> ViewStep for ViewSkip<S> {
    async fn pass(&mut self, line: &str) -> bool {
        self.steps.pass(line).await && self.counted()
    }
    fn fail(&mut self) -> bool {
        self.steps.fail() && self.counted()
    }
    fn over(&self) -> bool {
        self.steps.over()
    }
}

/// The first line and every `step`-th after it — `step_by`.
pub struct ViewStepBy<S> {
    steps: S,
    step: usize,
    seen: usize,
}

impl<S> ViewStepBy<S> {
    fn counted(&mut self) -> bool {
        let yes = self.seen.is_multiple_of(self.step);
        self.seen += 1;
        yes
    }
}

impl<S: ViewStep> ViewStep for ViewStepBy<S> {
    async fn pass(&mut self, line: &str) -> bool {
        self.steps.pass(line).await && self.counted()
    }
    fn fail(&mut self) -> bool {
        self.steps.fail() && self.counted()
    }
    fn over(&self) -> bool {
        self.steps.over()
    }
}

impl Views<Every> {
    /// The lines of `lines`, read into a pool of the least size: no line is
    /// held past the turn that sees it.
    fn of(lines: Lines) -> Views<Every> {
        Views {
            lines: lines.in_pool(2),
            steps: Every,
        }
    }
}

impl<S: ViewStep> Views<S> {
    /// **The next line that goes on, seen by `f`**, whose result is what the
    /// turn hands back: the line is a view of the pool for that call alone.
    async fn walk<U>(&mut self, f: &mut impl FnMut(&str) -> U) -> Option<Result<U, IoError>> {
        loop {
            if self.steps.over() {
                return None;
            }
            let (text, _, _) = match self.lines.next_line().await? {
                Ok(line) => line.parts(),
                Err(e) if self.steps.fail() => return Some(Err(e)),
                Err(_) => continue,
            };
            if self.steps.pass(text).await {
                return Some(Ok(f(text)));
            }
        }
    }

    /// The pool a `for` over the walk reads into, sized by the compiler's
    /// count for that loop (ADR-stream-buffers-v1 D3), as [`Lines::in_pool`].
    pub fn in_pool(mut self, size: i64) -> Views<S> {
        if let Some(pool) = self.lines.pool.as_mut() {
            pool.size = usize::try_from(size).unwrap_or(2).max(2);
        }
        self
    }

    /// **The next line that goes on, as a view** of the chunk it was cut
    /// from: what a `for` over the walk steps with, as
    /// [`Lines::next_line`].
    pub async fn next_line(&mut self) -> Option<Result<Line<'_>, IoError>> {
        loop {
            if self.steps.over() {
                return None;
            }
            let (passed, over) = match self.lines.next_line().await? {
                Ok(line) => (self.steps.pass(line.text).await, line.over),
                Err(e) if self.steps.fail() => return Some(Err(e)),
                Err(_) => continue,
            };
            if passed {
                return self.lines.again(over).map(Ok);
            }
            if let Some(size) = over {
                over_the_pool(size);
            }
        }
    }

    /// Each line that goes on through a lambda — `Seq::map`.
    pub fn map<U, F: FnMut(&str) -> U>(self, f: F) -> crate::seq::Failing<ViewMap<S, F>> {
        crate::seq::Failing(ViewMap { views: self, f })
    }

    /// Only the lines a lambda says yes to — `Seq::filter`.
    pub fn filter<F: FnMut(&&str) -> bool>(self, f: F) -> Views<ViewFilter<S, F>> {
        Views {
            lines: self.lines,
            steps: ViewFilter {
                steps: self.steps,
                f,
            },
        }
    }

    /// The first `n` lines — `Seq::take`.
    pub fn take(self, n: usize) -> Views<ViewTake<S>> {
        Views {
            lines: self.lines,
            steps: ViewTake {
                steps: self.steps,
                left: n,
            },
        }
    }

    /// Everything after the first `n` lines — `Seq::skip`.
    pub fn skip(self, n: usize) -> Views<ViewSkip<S>> {
        Views {
            lines: self.lines,
            steps: ViewSkip {
                steps: self.steps,
                left: n,
            },
        }
    }

    /// The first line and then every `step`-th after it — `Seq::step_by`; a
    /// step of `0` is `1`.
    pub fn step_by(self, step: usize) -> Views<ViewStepBy<S>> {
        Views {
            lines: self.lines,
            steps: ViewStepBy {
                steps: self.steps,
                step: step.max(1),
                seen: 0,
            },
        }
    }

    /// A copy of each line that goes on — `Seq::cloned`, the copy the
    /// program asks for (ADR-stream-buffers-v1 D10).
    #[allow(clippy::type_complexity)]
    pub fn cloned(self) -> crate::seq::Failing<ViewMap<S, fn(&str) -> crate::block::Text>> {
        self.map(copied as fn(&str) -> crate::block::Text)
    }

    /// How many lines go on — `Seq::count`.
    pub async fn count(mut self) -> Result<i64, IoError> {
        let mut n = 0_i64;
        while let Some(seen) = self.walk(&mut |_| ()).await {
            seen?;
            n += 1;
        }
        Ok(n)
    }

    /// Every line that goes on, with `separator` between them —
    /// `Seq::join`: the lines are copied into the one text it hands back.
    pub async fn join(mut self, separator: &str) -> Result<crate::block::Text, IoError> {
        let mut out = crate::block::Text::new();
        let mut first = true;
        let mut put = |line: &str| {
            if !first {
                out.push_str(separator);
            }
            out.push_str(line);
            first = false;
        };
        while let Some(seen) = self.walk(&mut put).await {
            seen?;
        }
        Ok(out)
    }

    /// The lines that go on folded into one value — `Seq::fold`.
    pub async fn fold<A, F: FnMut(A, &str) -> A>(
        mut self,
        init: A,
        mut f: F,
    ) -> Result<A, IoError> {
        let mut acc = init;
        loop {
            if self.steps.over() {
                return Ok(acc);
            }
            let (text, _, _) = match self.lines.next_line().await {
                None => return Ok(acc),
                Some(Ok(line)) => line.parts(),
                Some(Err(e)) if self.steps.fail() => return Err(e),
                Some(Err(_)) => continue,
            };
            if self.steps.pass(text).await {
                acc = f(acc, text);
            }
        }
    }

    /// Whether a lambda says yes to some line that goes on — `Seq::any`.
    pub async fn any<F: FnMut(&str) -> bool>(mut self, mut f: F) -> Result<bool, IoError> {
        while let Some(yes) = self.walk(&mut f).await {
            if yes? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a lambda says yes to every line that goes on — `Seq::all`.
    pub async fn all<F: FnMut(&str) -> bool>(mut self, mut f: F) -> Result<bool, IoError> {
        while let Some(yes) = self.walk(&mut f).await {
            if !yes? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// The line at a position among those that go on, copied, or nothing —
    /// `Seq::nth`.
    pub async fn nth(mut self, at: i64) -> Result<Option<crate::block::Text>, IoError> {
        let mut seen = 0_i64;
        let mut found = None;
        while let Some(step) = self
            .walk(&mut |line: &str| {
                if seen == at {
                    found = Some(crate::block::Text::from(line));
                }
                seen += 1;
            })
            .await
        {
            step?;
            if found.is_some() || at < 0 {
                break;
            }
        }
        Ok(found)
    }

    /// Pairs, a copy of each line that goes on and one item of `other` —
    /// `Seq::zip` (a pair is kept past its turn).
    #[allow(clippy::type_complexity)]
    pub fn zip<J: IntoIterator>(
        self,
        other: J,
    ) -> crate::seq::Failing<
        crate::seq::OkZip<ViewMap<S, fn(&str) -> crate::block::Text>, J::IntoIter>,
    > {
        self.cloned().zip(other)
    }

    /// Each line that goes on through a lambda that pauses — `Seq::map`
    /// where its lambda pauses (ADR-233 D1): the line stays in its buffer
    /// across the pause, since only the walk reads the pool.
    pub fn then<U, F: AsyncFnMut(&str) -> U>(self, f: F) -> crate::seq::Failing<ViewThen<S, F>> {
        crate::seq::Failing(ViewThen { views: self, f })
    }

    /// Only the lines a lambda that pauses says yes to — `Seq::filter` where
    /// its lambda pauses.
    pub fn then_filter<F: AsyncFnMut(&&str) -> bool>(self, f: F) -> Views<ViewThenFilter<S, F>> {
        Views {
            lines: self.lines,
            steps: ViewThenFilter {
                steps: self.steps,
                f,
            },
        }
    }
}

/// A line copied into text of its own: the step of `.cloned()`.
fn copied(line: &str) -> crate::block::Text {
    crate::block::Text::from(line)
}

/// **Each line of a [`Views`] through a lambda** that sees it as a view: the
/// step hands on what the lambda makes, which is the walk's own.
pub struct ViewMap<S, F> {
    views: Views<S>,
    f: F,
}

impl<S: ViewStep, U, F: FnMut(&str) -> U> crate::seq::Step for ViewMap<S, F> {
    type Item = Result<U, IoError>;
    async fn step(&mut self) -> Option<Result<U, IoError>> {
        self.views.walk(&mut self.f).await
    }
}

impl Held {
    /// A chunk appended, or the end where it is empty.
    ///
    /// What has been handed out is dropped before the new bytes go in, so the
    /// buffer is the size of what is *unread* rather than of everything the
    /// stream ever carried - which is what keeps a pipe larger than memory a
    /// program that does not grow (Part III, 17.1).
    fn append(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            self.ended = true;
            return;
        }
        self.held.drain(..self.at);
        self.at = 0;
        self.held.extend_from_slice(chunk);
    }

    /// The next `want` bytes, where that many are held.
    fn take_exact(&mut self, want: usize) -> Option<Vec<u8>> {
        let rest = &self.held[self.at..];
        if rest.len() < want {
            return None;
        }
        let bytes = rest[..want].to_vec();
        self.at += want;
        Some(bytes)
    }

    /// One line out of what is already held, where there is one.
    ///
    /// A whole line is one with its newline in hand; at the end of the stream
    /// the remainder is a line too, and `None` there means there is nothing
    /// left at all.
    fn take_a_line(&mut self) -> Option<Result<crate::block::Text, IoError>> {
        let rest = &self.held[self.at..];
        let (line, step) = match rest.iter().position(|b| *b == b'\n') {
            Some(at) => (&rest[..at], at + 1),
            None if self.ended && !rest.is_empty() => (rest, rest.len()),
            None => return None,
        };
        let line = match line.strip_suffix(b"\r") {
            Some(shorter) => shorter,
            None => line,
        };
        let line = crate::block::Text::from_utf8(crate::block::List::from(line));
        self.at += step;
        Some(line.map_err(|e| {
            IoError::of(
                std::io::Error::new(std::io::ErrorKind::InvalidData, e),
                STDIN,
            )
        }))
    }
}

impl Pool {
    /// An empty buffer of a chunk's size and room for a cut line.
    fn buffer() -> Vec<u8> {
        Vec::with_capacity(CHUNK + CHUNK / 4)
    }

    /// `bytes` put into a keep of their own.
    fn chunk(bytes: Vec<u8>) -> Chunk {
        let bytes = std::sync::Arc::new(bytes);
        let keep = std::sync::Arc::new(crate::tether::Keep::new());
        let (_, place) = keep.put_viewed_at(ChunkBytes(std::sync::Arc::clone(&bytes)));
        Chunk { keep, bytes, place }
    }

    /// The next whole line of the current chunk, as the range of its bytes
    /// without the line ending, where it holds one; at the end of the stream
    /// the rest is a line too.
    fn cut(&mut self) -> Option<(usize, usize)> {
        let bytes = &self.chunks[self.current].bytes;
        let rest = &bytes[self.at..];
        let (len, step) = match rest.iter().position(|b| *b == b'\n') {
            Some(at) => (at, at + 1),
            None if self.ended && !rest.is_empty() => (rest.len(), rest.len()),
            None => return None,
        };
        let from = self.at;
        let mut to = from + len;
        if to > from && bytes[to - 1] == b'\r' {
            to -= 1;
        }
        self.at += step;
        Some((from, to))
    }

    /// **The buffer the next read goes into**: the current chunk where no
    /// line is held in it any more, else another such, else a new one while
    /// the pool is not full. More than that is the compiler's fault (D9): one
    /// more buffer, and under `nikaia test` the next line says so.
    fn free(&mut self) -> usize {
        let unheld = |chunk: &Chunk| std::sync::Arc::strong_count(&chunk.keep) == 1;
        if unheld(&self.chunks[self.current]) {
            return self.current;
        }
        if let Some(at) = self.chunks.iter().position(unheld) {
            return at;
        }
        if self.chunks.len() >= self.size && under_test() {
            self.over = true;
        }
        self.chunks.push(Pool::chunk(Pool::buffer()));
        self.chunks.len() - 1
    }

    /// **One more read of standard input**, into a free buffer whose start is
    /// the line the current chunk ends in unfinished (D8): standard output
    /// flushed first, as [`refill`] does.
    async fn refill(&mut self) -> Result<(), IoError> {
        {
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        let into = self.free();
        let mut bytes = {
            let (current, chunk) = if into == self.current {
                (None, &mut self.chunks[into])
            } else if into < self.current {
                let (low, high) = self.chunks.split_at_mut(self.current);
                (Some(&high[0]), &mut low[into])
            } else {
                let (low, high) = self.chunks.split_at_mut(into);
                (Some(&low[self.current]), &mut high[0])
            };
            if let Some(keep) = std::sync::Arc::get_mut(&mut chunk.keep) {
                *keep = crate::tether::Keep::new();
            }
            let mut bytes = std::mem::take(
                std::sync::Arc::get_mut(&mut chunk.bytes)
                    .expect("a free chunk's bytes are the pool's alone"),
            );
            match current {
                None => {
                    bytes.drain(..self.at);
                }
                Some(current) => {
                    bytes.clear();
                    bytes.extend_from_slice(&current.bytes[self.at..]);
                }
            }
            bytes
        };
        let (back, read) =
            crate::rt::io::stdin_into(std::mem::take(&mut bytes), CHUNK, &self.reply, &self.answer)
                .await;
        let read = match read {
            Ok(read) => read,
            Err(e) => {
                // The stream is over either way: a reader that kept asking
                // after a failure would loop on it.
                self.ended = true;
                return Err(IoError::of(e, STDIN));
            }
        };
        if read == 0 {
            self.ended = true;
        }
        let chunk = &mut self.chunks[into];
        *std::sync::Arc::get_mut(&mut chunk.bytes)
            .expect("a free chunk's bytes are the pool's alone") = back;
        let keep = std::sync::Arc::get_mut(&mut chunk.keep)
            .expect("a free chunk's keep is the pool's alone");
        let (_, place) = keep.put_viewed_at(ChunkBytes(std::sync::Arc::clone(&chunk.bytes)));
        chunk.place = place;
        self.current = into;
        self.at = 0;
        Ok(())
    }
}

impl Drop for Lines {
    /// **What the loop left unread goes back to standard input's buffer**, so
    /// that a [`read_line`] or a second `io::lines()` after it starts where
    /// this one stopped.
    fn drop(&mut self) {
        let Some(pool) = self.pool.take() else {
            return;
        };
        let rest = &pool.chunks[pool.current].bytes[pool.at..];
        let ended = pool.ended;
        with_held(|held| {
            let mut back = rest.to_vec();
            back.extend_from_slice(&held.held[held.at..]);
            held.held = back;
            held.at = 0;
            held.ended = held.ended || ended;
        });
    }
}

impl crate::seq::Step for Lines {
    type Item = Result<crate::block::Text, IoError>;
    async fn step(&mut self) -> Option<Result<crate::block::Text, IoError>> {
        self.next().await
    }
}

/// **Each line of a [`Views`] through a lambda that pauses**, which sees it
/// as a view: the line stays in its buffer across the pause.
pub struct ViewThen<S, F> {
    views: Views<S>,
    f: F,
}

impl<S: ViewStep, U, F: AsyncFnMut(&str) -> U> crate::seq::Step for ViewThen<S, F> {
    type Item = Result<U, IoError>;
    async fn step(&mut self) -> Option<Result<U, IoError>> {
        loop {
            if self.views.steps.over() {
                return None;
            }
            let (text, _, _) = match self.views.lines.next_line().await? {
                Ok(line) => line.parts(),
                Err(e) if self.views.steps.fail() => return Some(Err(e)),
                Err(_) => continue,
            };
            if self.views.steps.pass(text).await {
                return Some(Ok((self.f)(text).await));
            }
        }
    }
}
