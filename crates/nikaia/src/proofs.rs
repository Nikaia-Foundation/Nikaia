// crates/nikaia/src/proofs.rs
//
// **`nikaia.proofs`: what a build uses is the recorded answer**
// ([ADR-270](../../docs/specification/adr/adr-270.md) D4, D19-D21, #448).
//
// The book, the questions and their answers are `proofs_answers.nika`'s. What
// stays here is the thread they live on: a build's check and lowering run on
// one thread, and [`with_book`], [`in_file`] and [`in_function`] open what
// every [`ask`] on it sees, around a closure. Outside a book - a test,
// `nikaia lower` of a loose file - [`ask`] searches and records nothing.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nikaia_std::rt::exec::block_on;
use nikaia_std::tools::proofs_answers::{self as answers, ProofsBook, ProofsThread};
use nikaia_std::tools::prover_arena::TermArena;

pub use nikaia_std::tools::proofs_answers::{
    PROOFS_FILE as FILE, ProofsAsked as Asked, ProofsSoFar as AskedSoFar,
};

/// The answers a build reads and writes (`ProofsBook`), with the file's I/O.
#[derive(Debug, Clone)]
pub struct Book(ProofsBook);

impl Default for Book {
    fn default() -> Self {
        Book(answers::proofs_book_empty())
    }
}

impl std::ops::Deref for Book {
    type Target = ProofsBook;
    fn deref(&self) -> &ProofsBook {
        &self.0
    }
}

impl Book {
    /// The book `root`'s `nikaia.proofs` holds, or an empty one.
    pub fn read(root: &Path) -> Book {
        Book(block_on(answers::proofs_book_read(root)))
    }

    /// Write the file and its long certificates under `root`, where they
    /// changed, whole or not at all, as the ledger beside it
    /// (`project::write_ledger`).
    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        let whole =
            |path: &Path, text: &str| crate::comptime_run::write_if_changed(path, text).err();
        match answers::proofs_book_write(&self.0, root, &whole) {
            None => Ok(()),
            Some(failed) => Err(std::io::Error::other(failed)),
        }
    }

    /// Whether `root` holds exactly the file this book would write (D22).
    pub fn as_recorded(&self, root: &Path) -> bool {
        block_on(answers::proofs_book_as_recorded(&self.0, root))
    }

    /// How many entries the file would hold.
    pub fn len(&self) -> usize {
        answers::proofs_book_len(&self.0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Where a searched question was asked (D22): the file, the line and the
/// function, shown relative to `root`.
pub fn shown(place: &(Option<String>, Option<i64>, Option<String>), root: &Path) -> String {
    answers::proofs_shown(place, root)
}

thread_local! {
    static THREAD: RefCell<ProofsThread> = RefCell::new(answers::proofs_thread());
    static UNITS: RefCell<Vec<(PathBuf, String)>> = const { RefCell::new(Vec::new()) };
}

/// `run` with the files of a package named in order, so that a pass over
/// the package's units by their place ([`in_unit`]) asks each question in
/// its file.
pub fn with_units<R>(units: Vec<(PathBuf, String)>, run: impl FnOnce() -> R) -> R {
    let outer = UNITS.with(|held| held.replace(units));
    let out = run();
    UNITS.with(|held| held.replace(outer));
    out
}

/// `run` in the file [`with_units`] names at place `at`, or as it is where
/// it names none.
pub fn in_unit<R>(at: usize, run: impl FnOnce() -> R) -> R {
    match UNITS.with(|held| held.borrow().get(at).cloned()) {
        Some((path, source)) => in_file(&path, &source, run),
        None => run(),
    }
}

/// `run` with every question it asks placed in `path`, whose text is
/// `source`.
pub fn in_file<R>(path: &Path, source: &str, run: impl FnOnce() -> R) -> R {
    let here = answers::proofs_where(path, source);
    let outer = THREAD.with(|t| std::mem::replace(&mut t.borrow_mut().at, here));
    let out = run();
    THREAD.with(|t| t.borrow_mut().at = outer);
    out
}

/// **`run` for one file of the program**: its questions placed in `path`,
/// and - for a dependency's file - answered from that package's own
/// `nikaia.proofs` and kept out of this build's (D23).
pub fn for_unit<R>(path: &Path, source: &str, dependency: bool, run: impl FnOnce() -> R) -> R {
    if !dependency {
        return in_file(path, source, run);
    }
    let theirs = orchestrator::cache::Layout::resolve(path);
    let book = match theirs.in_project {
        true => Book::read(&theirs.root),
        false => Book::default(),
    };
    with_book(book, || in_file(path, source, run)).0
}

/// `run` with every question it asks placed in the function `name`, whose
/// declaration starts at byte `at` of the file.
pub fn in_function<R>(name: &str, at: usize, run: impl FnOnce() -> R) -> R {
    let outer = THREAD.with(|t| {
        let mut t = t.borrow_mut();
        let name = t.function.replace(name.to_string());
        (name, std::mem::replace(&mut t.function_at, at as i64))
    });
    let out = run();
    THREAD.with(|t| {
        let mut t = t.borrow_mut();
        (t.function, t.function_at) = outer;
    });
    out
}

/// `run` with `book` open for every [`ask`] on this thread, and the book as
/// it stands after.
pub fn with_book<R>(book: Book, run: impl FnOnce() -> R) -> (R, Book) {
    let outer = THREAD.with(|t| t.borrow_mut().book.replace(book.0));
    let out = run();
    let book = THREAD.with(|t| std::mem::replace(&mut t.borrow_mut().book, outer));
    (out, book.map(Book).unwrap_or_default())
}

/// Whether the facts imply the goal, and if not, why not.
pub fn ask(arena: &TermArena, facts: &[i64], goal: i64) -> Asked {
    THREAD.with(|t| answers::proofs_ask(&mut t.borrow_mut(), arena, facts, goal))
}

/// The invariants the book holds for the file being checked (ADR-361 D4).
pub fn invariants_here() -> BTreeMap<i64, String> {
    THREAD.with(|t| answers::proofs_invariants_here(&t.borrow()))
}

/// Keep the invariants a walk of the file being checked used (ADR-361 D4).
pub fn record_invariants(used: &BTreeMap<i64, String>) {
    THREAD.with(|t| answers::proofs_record_at(&mut t.borrow_mut(), used, ""));
}

/// Keep the invariants the bounds walk of the file being lowered used
/// (ADR-361 D10, D4), under `/bounds` beside the prover's own.
pub fn record_bound_invariants(used: &BTreeMap<i64, String>) {
    if !used.is_empty() {
        THREAD.with(|t| answers::proofs_record_at(&mut t.borrow_mut(), used, "/bounds"));
    }
}

/// Whether a build's book is open on this thread.
pub fn book_open() -> bool {
    THREAD.with(|t| t.borrow().book.is_some())
}

/// What the open book has been asked so far.
pub fn asked_so_far() -> AskedSoFar {
    THREAD.with(|t| answers::proofs_asked_so_far(&t.borrow()))
}

/// Forget what was asked after `before`: a walk that is repeated keeps only
/// what its repetition asks.
pub fn ask_again_from(before: &AskedSoFar) {
    THREAD.with(|t| answers::proofs_ask_again_from(&mut t.borrow_mut(), before));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `0 <= i < n <= len` ⊢ `i < len`, and `x >= 3` ⊢ `x > 10`.
    fn ask_two() {
        let mut a = TermArena::empty();
        let (i, n, len, zero) = (a.var("i"), a.var("n"), a.var("len"), a.int(0));
        let facts = vec![a.ge(i, zero), a.lt(i, n), a.le(n, len)];
        let goal = a.lt(i, len);
        assert_eq!(ask(&a, &facts, goal), Asked::Proved);
        let (x, three, ten) = (a.var("x"), a.int(3), a.int(10));
        let low = vec![a.ge(x, three)];
        let high = a.gt(x, ten);
        let Asked::Refuted(model) = ask(&a, &low, high) else {
            panic!("x = 3 is a model");
        };
        assert_eq!(model.values.get("x"), Some(&3), "named back: {model:?}");
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nikaia-proofs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    /// **D21**: what is missing is searched and recorded; **D4**: a second
    /// build with the file searches nothing.
    #[test]
    fn a_recorded_answer_is_read_and_not_searched_again() {
        let ((), first) = with_book(Book::default(), ask_two);
        assert_eq!((first.searched, first.len()), (2, 2));
        let dir = scratch("read");
        first.write(&dir).expect("written");
        let ((), second) = with_book(Book::read(&dir), ask_two);
        assert_eq!(second.searched, 0);
        assert_eq!(
            answers::proofs_book_render(&second),
            answers::proofs_book_render(&first)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **D20**: an entry that does not check counts as missing - searched
    /// again and written right - and an entry nothing asked is gone.
    #[test]
    fn a_wrong_entry_is_searched_again_and_an_unasked_one_dropped() {
        let ((), first) = with_book(Book::default(), ask_two);
        let dir = scratch("wrong");
        first.write(&dir).expect("written");
        let text = std::fs::read_to_string(dir.join(FILE)).expect("the file");
        let broken = text.replace(" refuted v0=3", " refuted v0=11").replacen(
            " proved R(",
            " proved R(h9,",
            1,
        ) + &format!("{} proved R(h0)\n", "0".repeat(64));
        std::fs::write(dir.join(FILE), broken).expect("rewritten");
        let ((), again) = with_book(Book::read(&dir), ask_two);
        assert_eq!(again.searched, 2);
        assert_eq!(
            answers::proofs_book_render(&again),
            answers::proofs_book_render(&first)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **ADR-361 D4**: a loop's place is its line and column, read back to
    /// the same byte; one past its line's end is nothing.
    #[test]
    fn a_loop_place_reads_back_to_its_byte() {
        let source = "fn f() {\n    while x {\n    }\n}\n";
        let at = source.find("while").expect("the loop");
        assert_eq!(answers::proofs_place_of(at as i64, source), "2:5");
        assert_eq!(answers::proofs_byte_of("2:5", source), Some(at as i64));
        assert_eq!(answers::proofs_byte_of("2:40", source), None);
        assert_eq!(answers::proofs_byte_of("9:1", source), None);
    }

    /// Outside a build nothing is recorded, and the answer is the same.
    #[test]
    fn without_a_book_a_question_is_searched() {
        ask_two();
    }

    /// **D22**: a searched question keeps where it was asked - the file, and
    /// the function with its line where the asker said it.
    #[test]
    fn a_searched_question_keeps_its_place() {
        let root = Path::new("/p");
        let source = "// one\nfn f() {}\n";
        let ((), book) = with_book(Book::default(), || {
            in_file(&root.join("src/a.nika"), source, || {
                in_function("f", source.find("fn").expect("the fn"), ask_two)
            })
        });
        let shown: Vec<String> = book
            .searched_at
            .iter()
            .map(|p| super::shown(p, root))
            .collect();
        assert_eq!(shown, ["src/a.nika:2, in `f`"]);
        let ((), book) = with_book(Book::default(), ask_two);
        let shown: Vec<String> = book
            .searched_at
            .iter()
            .map(|p| super::shown(p, root))
            .collect();
        assert_eq!(shown, ["a place the compiler did not name"]);
    }
}
