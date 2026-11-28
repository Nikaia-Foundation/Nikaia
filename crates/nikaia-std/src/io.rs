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
    Lines { _shared: () }
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
    /// **The eager walks**: what `collect`, `count`, `nth` and `join` hand back
    /// is a value, so each is a loop around [`Lines::next`] and nothing more.
    /// The lazy ones hand back another sequence whose step pauses and can
    /// fail, `nikaia_std::seq::Failing`
    /// ([ADR-234](../../../docs/specification/adr/adr-234.md)).
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

    /// How many lines the sequence produces — `Seq::count`.
    ///
    /// It walks the whole of it to answer, which is what separates this from a
    /// list's `len()`, and it holds one line at a time while it does.
    pub async fn count(mut self) -> Result<i64, IoError> {
        let mut n = 0_i64;
        while let Some(line) = self.next().await {
            line?;
            n += 1;
        }
        Ok(n)
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
    /// `Seq::join`.
    pub async fn join(mut self, separator: &str) -> Result<crate::block::Text, IoError> {
        let mut out = crate::block::Text::new();
        let mut first = true;
        while let Some(line) = self.next().await {
            if !first {
                out.push_str(separator);
            }
            out.push_str(&line?);
            first = false;
        }
        Ok(out)
    }

    /// Each line through a lambda — `Seq::map`, lazy: nothing is read until
    /// something walks the result
    /// ([ADR-234](../../../docs/specification/adr/adr-234.md) D1). A failure
    /// passes through to that walk, which is where the program's `?` is.
    pub fn map<U, F: FnMut(crate::block::Text) -> U>(
        self,
        f: F,
    ) -> crate::seq::Failing<crate::seq::OkMap<Self, F>> {
        crate::seq::Failing(self).map(f)
    }

    /// Only the lines a lambda says yes to — `Seq::filter`, lazy.
    pub fn filter<F: FnMut(&crate::block::Text) -> bool>(
        self,
        f: F,
    ) -> crate::seq::Failing<crate::seq::OkFilter<Self, F>> {
        crate::seq::Failing(self).filter(f)
    }

    /// Each line through a lambda that pauses
    /// ([ADR-233](../../../docs/specification/adr/adr-233.md) D1).
    pub fn then<U, F: AsyncFnMut(crate::block::Text) -> U>(
        self,
        f: F,
    ) -> crate::seq::Failing<crate::seq::OkThen<Self, F>> {
        crate::seq::Failing(self).then(f)
    }

    /// Only the lines a lambda that pauses says yes to.
    pub fn then_filter<F: AsyncFnMut(&crate::block::Text) -> bool>(
        self,
        f: F,
    ) -> crate::seq::Failing<crate::seq::OkThenFilter<Self, F>> {
        crate::seq::Failing(self).then_filter(f)
    }

    /// The first `n` lines — `Seq::take`, lazy.
    pub fn take(self, n: usize) -> crate::seq::Failing<crate::seq::Take<Self>> {
        crate::seq::Failing(self).take(n)
    }

    /// Everything after the first `n` lines — `Seq::skip`, lazy.
    pub fn skip(self, n: usize) -> crate::seq::Failing<crate::seq::Skip<Self>> {
        crate::seq::Failing(self).skip(n)
    }

    /// The first line, and then every `step`-th after it — `Seq::step_by`.
    pub fn step_by(self, step: usize) -> crate::seq::Failing<crate::seq::StepBy<Self>> {
        crate::seq::Failing(self).step_by(step)
    }

    /// Pairs, one line and one item of `other` — `Seq::zip`, lazy.
    pub fn zip<J: IntoIterator>(
        self,
        other: J,
    ) -> crate::seq::Failing<crate::seq::OkZip<Self, J::IntoIter>> {
        crate::seq::Failing(self).zip(other)
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

impl crate::seq::Step for Lines {
    type Item = Result<crate::block::Text, IoError>;
    async fn step(&mut self) -> Option<Result<crate::block::Text, IoError>> {
        self.next().await
    }
}
