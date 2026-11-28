//! `std::fs` - files.

use std::path::Path as FilePath;

/// **A file's name, in the platform's own bytes**
/// ([ADR-319](../../../docs/specification/adr/adr-319.md) D1): raw bytes on
/// Unix, WTF-8 on Windows. Made from text, by the system, or by its own
/// operations, so no call fails for its encoding. A view of one, `ref
/// fs::Path`, is a `&std::path::Path`, which text lends without a copy (D2).
pub type Path = std::path::PathBuf;

/// **What a program does with a name, it does on the `Path`**
/// ([ADR-319](../../../docs/specification/adr/adr-319.md) D3). Functions, and
/// not methods, because Rust's `Path` has methods by these names that mean
/// something else: its `ends_with` compares whole components, so
/// `"data.csv".ends_with(".csv")` would be false. These compare the
/// platform's bytes, and every result keeps D1's validity.
pub mod path {
    use std::path::{Path as FilePath, PathBuf};

    fn bytes(p: &FilePath) -> &[u8] {
        p.as_os_str().as_encoded_bytes()
    }

    /// Whether the name begins with these bytes.
    pub fn starts_with(p: &FilePath, prefix: &FilePath) -> bool {
        bytes(p).starts_with(bytes(prefix))
    }

    /// Whether the name ends with these bytes: `".csv"` is a suffix of
    /// `data.csv`.
    pub fn ends_with(p: &FilePath, suffix: &FilePath) -> bool {
        bytes(p).ends_with(bytes(suffix))
    }

    /// The last part of the name, where there is one.
    pub fn file_name(p: &FilePath) -> Option<PathBuf> {
        p.file_name().map(PathBuf::from)
    }

    /// What follows the last `.` of the last part, where there is one.
    pub fn extension(p: &FilePath) -> Option<PathBuf> {
        p.extension().map(PathBuf::from)
    }

    /// The name without its last part, where it has one.
    pub fn parent(p: &FilePath) -> Option<PathBuf> {
        p.parent().map(FilePath::to_path_buf)
    }

    /// The same name with another extension; its bytes are kept as they are.
    pub fn with_extension(p: &FilePath, extension: &FilePath) -> PathBuf {
        p.with_extension(extension.as_os_str())
    }

    /// **The rest of the name after `base`'s whole parts** (ADR-319 D3,
    /// amended 2026-10-28), or `None` where they do not lead it: `a/b/c`
    /// after `a` is `b/c`, `a/b` after `a/b` the empty name, and `ab/c` after
    /// `a` is `None`. It reads no file system.
    pub fn strip_prefix(p: &FilePath, base: &FilePath) -> Option<PathBuf> {
        p.strip_prefix(base).ok().map(FilePath::to_path_buf)
    }

    /// **The whole parts of the name, in order** (ADR-319 D10): the root (on
    /// Windows the prefix, then the root) first where there is one, `..` as
    /// written, a `.` only where it leads; empty parts and a `.` inside are
    /// dropped. Joined again they give a name [`same`] as `p`. It reads no
    /// file system. A block's list, as a program's `Vec[fs::Path]` is: a
    /// plain `Vec` was a rustc error where the result met one (#1619).
    pub fn parts(p: &FilePath) -> crate::block::List<PathBuf> {
        let parts: Vec<PathBuf> = p
            .components()
            .map(|part| PathBuf::from(part.as_os_str()))
            .collect();
        crate::block::List::from(parts)
    }

    /// A copy of the name, owned by whoever asked for it: how a lent name is
    /// kept beyond the call that lent it.
    pub fn clone(p: &FilePath) -> PathBuf {
        p.to_path_buf()
    }

    /// The name with another part after it.
    pub fn join(p: &FilePath, part: &FilePath) -> PathBuf {
        p.join(part)
    }

    /// **The same name, part by part** (ADR-319 D3, D9), against a name or
    /// against text: whole parts are compared one after the other, each by
    /// its bytes, so `a//b`, `a/./b` and `a/b` are one name, as they sort
    /// together and as [`parts`] hands them out.
    pub fn same<A, B>(a: &A, b: &B) -> bool
    where
        A: AsRef<std::ffi::OsStr> + ?Sized,
        B: AsRef<std::ffi::OsStr> + ?Sized,
    {
        FilePath::new(a.as_ref()) == FilePath::new(b.as_ref())
    }

    /// **An `f"…"` with a name in it, joined in the platform's encoding**
    /// (ADR-319 D4): a name keeps its bytes, and text is added as text.
    pub fn joined(parts: &[&std::ffi::OsStr]) -> PathBuf {
        let mut name = std::ffi::OsString::new();
        for part in parts {
            name.push(part);
        }
        PathBuf::from(name)
    }

    /// **A name printed as it is** (ADR-319 D4): its bytes, to standard
    /// output or standard error, with a newline where the call says so.
    ///
    /// A failed write is an `IoError`, as every print's is (ADR-392).
    pub fn print(p: &FilePath, to_err: bool, newline: bool) -> Result<(), crate::io::IoError> {
        crate::io::print_bytes(bytes(p), to_err, newline)
    }

    /// **Text for showing** (ADR-319 D6): `�` for what is not text. It never
    /// fails, and nothing tracks what is done with it.
    /// The name as text (ADR-319 D5): `NotText` where its bytes are not.
    pub fn to_text(p: &FilePath) -> Result<crate::block::Text, crate::io::IoError> {
        match p.to_str() {
            Some(text) => Ok(crate::block::Text::from(text)),
            None => Err(crate::io::IoError::NotText(p.display().to_string())),
        }
    }

    /// The name as text, where the compiler followed it back to text (D5):
    /// made from text, it is text, so nothing is lost.
    pub fn text_of(p: &FilePath) -> crate::block::Text {
        crate::block::Text::from(&*p.to_string_lossy())
    }

    pub fn display(p: &FilePath) -> crate::block::Text {
        crate::block::Text::from(&*p.to_string_lossy())
    }
}

/// **Where a path may go** ([ADR-108](../../../docs/specification/adr/adr-108.md)
/// D2): the directory the name is resolved under, or the word that says the
/// program answers for the name itself.
///
/// Every path-taking entry of this module takes one, right after the path, with
/// **no default** (D1). That is Part I 5.1's rule doing the work: an option has a
/// default, and a default here is the hole — a call that could leave the root out
/// is a call nobody can be sure remembered it. A call that leaves it out is
/// `NK1101`, the same refusal as any other missing argument.
///
/// Two variants and no shorthand (D2). Not a bare `String` where a `Root` is
/// wanted, which would be a conversion rule this language does not have; and not
/// a second spelling beside the variant, which would be two ways to write one
/// thing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Root {
    /// The name is resolved under this directory and may not leave it.
    ///
    /// A name, in the platform's own bytes (ADR-319 D7); text stands where
    /// one is asked, so `fs::Root::Dir("site")` is written as it always was.
    Dir(Path),
    /// No check — the program answers for the name.
    ///
    /// It does not say *checked*. It says *I answer for this*, and it is visible
    /// three times (D4): at the line, in the ledger, and in `nikaia --trust`.
    Anywhere,
}

/// The name this call hands the operating system, or the refusal that it leaves
/// its root.
///
/// **Resolved and compared by component, never as a string prefix** (D3),
/// because `/data` is a prefix of `/data2`. Both sides are canonicalised —
/// symlinks followed, `..` applied — which is what makes a symlink out of the
/// directory the same answer as a `..` out of it.
///
/// **What comes back is the name joined under the root, and not the resolved
/// one.** The resolution is for the *comparison*; what the operating system is
/// given is what the program asked for, under the directory it was given. D3's
/// *never a rewritten name* is the reason, and the case that shows it is
/// `sub/../ok.txt` where `sub` does not exist: resolved it lands on `ok.txt`,
/// and handing that over would serve a file a POSIX `open` of the written name
/// says is not there. So the check answers *inside*, and the operating system
/// answers *not found* — each about the thing it knows.
///
/// **A name that does not exist yet is checked all the same.** `fs::write`
/// creates, so canonicalising the whole name would fail on exactly the call that
/// most needs the check; the walk below canonicalises every component that
/// exists and applies the rest on top.
///
/// **`Anywhere` resolves nothing**, which is the whole of what it costs at run
/// time: the name goes to the operating system exactly as the program wrote it.
fn resolve(path: &FilePath, root: &Root) -> Result<std::path::PathBuf, crate::io::IoError> {
    let Root::Dir(dir) = root else {
        return Ok(path.to_path_buf());
    };
    // The name is written out only for the refusal (ADR-263 D3).
    let outside = || crate::io::IoError::Outside(path.to_path_buf());

    // The root itself has to resolve. One that does not is not a directory a
    // name may be used under, and answering `Outside` for it is the fail-closed
    // direction ([ADR-010](../../../docs/specification/adr/adr-010.md) D1).
    let base = std::fs::canonicalize(FilePath::new(dir)).map_err(|_| outside())?;

    // **An absolute name gets the same treatment**: resolved, compared, and fine
    // where it lies inside. A relative one is joined under the root, which is
    // what makes the root this call's working directory and closes D1's
    // *whoever controls the working directory has changed what the name means*.
    let joined = match path.is_absolute() {
        true => path.to_path_buf(),
        false => base.join(path),
    };
    let landed = climb(&joined).ok_or_else(outside)?;
    match landed.starts_with(&base) {
        true => Ok(joined),
        false => Err(outside()),
    }
}

/// `path` with every symlink followed and every `..` applied, for a name whose
/// tail may not exist yet.
///
/// `std::fs::canonicalize` answers only for a name that is there, and a write
/// creates. So this walks the components: each ordinary one is pushed and the
/// result canonicalised **where it exists**, so a symlink is followed the moment
/// it can be; a `.` is dropped; and a `..` pops.
///
/// **A `..` with nothing to pop is `None`**, because a name that climbs above the
/// filesystem root has left every directory it could have been joined to. The
/// caller reads that as *outside*.
fn climb(path: &FilePath) -> Option<std::path::PathBuf> {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => {
                out.push(other);
                if let Ok(real) = std::fs::canonicalize(&out) {
                    out = real;
                }
            }
        }
    }
    Some(out)
}

/// **Whether a name is there** (Part III 17.1): a file, a directory, or a
/// link that leads to one.
///
/// **The root is checked first** ([ADR-108](../../../docs/specification/adr/adr-108.md)
/// D1): a name that leaves a `Root::Dir` - by `..`, or a link out of it - is
/// refused with `Outside` and not answered `false`, because an outside name is
/// refused, not absent, and the call `throws` (the fail-closed direction of
/// [`resolve`]).
///
/// `std::fs::exists` and not `Path::exists`, which answers `false` where the
/// operating system refused to say: a directory that may not be read is
/// `PermissionDenied`.
///
/// **Not `async`**: one `stat`, the same size of blocking as [`scratch`]'s one
/// system call, so it does not go through the I/O reactor as a read does.
pub fn exists(path: impl AsRef<FilePath>, root: &Root) -> Result<bool, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    std::fs::exists(&resolved).map_err(|e| crate::io::IoError::of(e, asked))
}

/// **The absolute name, with every link, `.` and `..` resolved** (Part III
/// 17.1, ADR-319 D3 amended 2026-10-28). The name has to exist: `NotFound`
/// otherwise. Under `Root::Dir(store)` a name whose resolution leaves the
/// store's own canonical name is `Outside`, as for [`exists`].
///
/// **Not `async`**, as `exists` is not: one system call.
pub fn canonical(
    path: impl AsRef<FilePath>,
    root: &Root,
) -> Result<std::path::PathBuf, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let found = std::fs::canonicalize(&resolved).map_err(|e| crate::io::IoError::of(e, asked))?;
    if let Root::Dir(dir) = root {
        let outside = || crate::io::IoError::Outside(asked.to_path_buf());
        let base = std::fs::canonicalize(FilePath::new(dir)).map_err(|_| outside())?;
        if !found.starts_with(&base) {
            return Err(outside());
        }
    }
    Ok(found)
}

/// **Making, removing, renaming and copying names** (Part III 17.1, #546).
///
/// Each resolves its names under the root as [`exists`] does, so a name that
/// leaves a `Root::Dir` is `Outside` before anything is touched; `rename` and
/// `copy` resolve both names under the one root they are given.
///
/// **Not `async`**, as `exists` and `scratch` are not: each is one system call,
/// or a walk of one tree for `recursive`, and none of it goes through the I/O
/// reactor.
pub fn create_dir(
    path: impl AsRef<FilePath>,
    root: &Root,
    recursive: bool,
) -> Result<(), crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let made = match recursive {
        true => std::fs::create_dir_all(&resolved),
        false => std::fs::create_dir(&resolved),
    };
    made.map_err(|e| crate::io::IoError::of(e, asked))
}

/// **A file, or a directory**: an empty one, or with everything in it where
/// `recursive` says so. A link is removed, never what it leads to.
pub fn remove(
    path: impl AsRef<FilePath>,
    root: &Root,
    recursive: bool,
) -> Result<(), crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let of = |e| crate::io::IoError::of(e, asked);
    let kind = std::fs::symlink_metadata(&resolved).map_err(of)?;
    let removed = match (kind.is_dir(), recursive) {
        (true, true) => std::fs::remove_dir_all(&resolved),
        (true, false) => std::fs::remove_dir(&resolved),
        (false, _) => std::fs::remove_file(&resolved),
    };
    removed.map_err(of)
}

/// **A name given to what another had**, both under `root`.
pub fn rename(
    from: impl AsRef<FilePath>,
    to: impl AsRef<FilePath>,
    root: &Root,
) -> Result<(), crate::io::IoError> {
    let (from, to) = (from.as_ref(), to.as_ref());
    let source = resolve(from, root)?;
    let target = resolve(to, root)?;
    std::fs::rename(&source, &target).map_err(|e| crate::io::IoError::of(e, from))
}

/// **A file's bytes, written to another name** under the same root; how many
/// were copied.
pub fn copy(
    from: impl AsRef<FilePath>,
    to: impl AsRef<FilePath>,
    root: &Root,
) -> Result<u64, crate::io::IoError> {
    let (from, to) = (from.as_ref(), to.as_ref());
    let source = resolve(from, root)?;
    let target = resolve(to, root)?;
    std::fs::copy(&source, &target).map_err(|e| crate::io::IoError::of(e, from))
}

/// **A file's bytes, written from one root into another**, its permission bits
/// kept; how many were copied. A platform primitive (ADR-platform-layer-v1
/// D2, D4, D7 item 1): each name resolved under its own root, one copy.
pub fn copy_between(
    from: impl AsRef<FilePath>,
    from_root: &Root,
    to: impl AsRef<FilePath>,
    to_root: &Root,
) -> Result<u64, crate::io::IoError> {
    let (from, to) = (from.as_ref(), to.as_ref());
    let source = resolve(from, from_root)?;
    let target = resolve(to, to_root)?;
    std::fs::copy(&source, &target).map_err(|e| crate::io::IoError::of(e, from))
}

/// **What the system knows of a name** (Part III 17.1): its length and what it
/// is, following a link to what it leads to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    len: u64,
    is_dir: bool,
    is_file: bool,
    modified: std::time::Duration,
    links: u64,
    file_id: Option<(u64, u64)>,
}

impl Metadata {
    /// Its length in bytes, an `i64` as a length is (Part I 2.2).
    ///
    /// No `is_empty`: Part III 17.1 names `len`, `is_dir` and `is_file`, and a
    /// file of no bytes is `len() == 0`.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> i64 {
        i64::try_from(self.len).unwrap_or(i64::MAX)
    }

    /// Whether it is a directory.
    pub fn is_dir(&self) -> bool {
        self.is_dir
    }

    /// Whether it is a file.
    pub fn is_file(&self) -> bool {
        self.is_file
    }

    /// The last modification, as the span since the Unix epoch (#922): the
    /// wall clock's reading, comparable with `time::since_epoch()`; zero where
    /// the platform does not say or the time is before the epoch.
    pub fn modified(&self) -> std::time::Duration {
        self.modified
    }

    /// The number of hard links to it (#922); `1` off Unix.
    pub fn links(&self) -> u64 {
        self.links
    }

    /// Its device and inode (#922), which two names of one file share; null
    /// off Unix.
    pub fn file_id(&self) -> Option<(u64, u64)> {
        self.file_id
    }
}

/// The hard links and the device and inode of what `known` describes: the
/// numbers Unix keeps (`MetadataExt`).
#[cfg(unix)]
fn links_and_id(known: &std::fs::Metadata) -> (u64, Option<(u64, u64)>) {
    use std::os::unix::fs::MetadataExt;
    (known.nlink(), Some((known.dev(), known.ino())))
}

/// Off Unix: one link and no identity, as `Metadata::links` and `file_id` say.
#[cfg(not(unix))]
fn links_and_id(_known: &std::fs::Metadata) -> (u64, Option<(u64, u64)>) {
    (1, None)
}

/// What the system knows of the name, which [`Metadata`] holds.
pub fn metadata(path: impl AsRef<FilePath>, root: &Root) -> Result<Metadata, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let known = std::fs::metadata(&resolved).map_err(|e| crate::io::IoError::of(e, asked))?;
    let modified = known
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .unwrap_or_default();
    let (links, file_id) = links_and_id(&known);
    Ok(Metadata {
        len: known.len(),
        is_dir: known.is_dir(),
        is_file: known.is_file(),
        modified,
        links,
        file_id,
    })
}

/// **The names in a directory**, each its own name and not joined to the
/// directory's, in byte order - so a program that prints them prints the same
/// lines on every machine, whatever order the system keeps them in.
pub type DirEntries = crate::block::List<Path>;

/// The names in the directory, as [`DirEntries`].
pub fn read_dir(path: impl AsRef<FilePath>, root: &Root) -> Result<DirEntries, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let of = |e| crate::io::IoError::of(e, asked);
    let mut names = crate::block::List::new();
    for entry in std::fs::read_dir(&resolved).map_err(of)? {
        names.push(Path::from(entry.map_err(of)?.file_name()));
    }
    names.sort();
    Ok(names)
}

/// A file's bytes, addressable for as long as the value lives.
///
/// Part III, 17.1: a memory mapping. The pages *are* the buffer, so a 13 GB
/// input costs no copy and every view a parser yields points straight into the
/// file. **The pages are bytes** ([ADR-016](../../../docs/specification/adr/adr-016.md)
/// D5): mapping checks nothing, and [`Mapped::text`] checks the whole of it
/// as UTF-8 the first time it is asked for and never again.
///
/// ADR-grammar-input-v1 D13 allows a large read-only mapping to be left to the OS at process
/// exit. Nothing here does that yet: the mapping is released when the value is
/// dropped, and the drop is what `main` returning costs.
///
/// **A mapping is a `Bytes`** ([ADR-373](../../../docs/specification/adr/adr-373.md)
/// D3): the pages are held in an `Arc`, so a `Bytes` made from a mapping
/// ([`Mapped::bytes`], `Bytes::from`) is the mapping at one more count, and
/// the file stays mapped while any of them lives.
///
/// **A copy is the same mapping at one more count**, as a `Bytes`'s is: a
/// struct that holds one (Part III's `State[fs::Mapped]`) is written with the
/// `Debug, Clone` every struct is lowered with (#2218).
#[derive(Clone)]
pub struct Mapped {
    pages: std::sync::Arc<Pages>,
}

/// **What a mapping derefs to**: its bytes, read as any `Bytes` is read
/// (`b[i]`, a cut, `len`, `find`, `u64_le`), and its text, checked once.
///
/// A type of its own rather than `[u8]` so that a grammar entered with a
/// mapping (`&*data`, [`crate::grammar::parse`]) still knows it holds a
/// mapping, and asks for its text through the one check the mapping keeps.
pub struct Pages {
    map: Backing,
    /// What the program asked to map, for a `NotText` to name.
    asked: std::path::PathBuf,
}

enum Backing {
    Pages(checked_text::OnDemand<checked_text::Mmap>),
    /// A zero-length file cannot be mapped, and an empty input is still an
    /// input.
    Empty,
}

impl Pages {
    /// The mapping as text, checked the first time only, or the first byte
    /// that is not UTF-8.
    pub(crate) fn checked(&self) -> Result<&str, usize> {
        match &self.map {
            Backing::Pages(pages) => pages.text(),
            Backing::Empty => Ok(""),
        }
    }

    /// The mapping as text: `NotText` naming the file and the earliest bad
    /// byte, the failure `fs::read_to_string` reports for the same file.
    pub fn text(&self) -> Result<&str, crate::io::IoError> {
        self.checked().map_err(|at| {
            crate::io::IoError::NotText(format!("{} (at byte {at})", self.asked.display()))
        })
    }
}

impl std::ops::Deref for Pages {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        match &self.map {
            Backing::Pages(pages) => pages.bytes(),
            Backing::Empty => &[],
        }
    }
}

/// The pages lend their bytes, so a `Bytes` can own them (ADR-373 D3).
impl AsRef<[u8]> for Pages {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

/// **A mapping handed where a `Bytes` is taken** (ADR-373 D3): the mapping,
/// at a count; nothing is copied.
impl From<&Mapped> for crate::bytes::Bytes {
    fn from(mapped: &Mapped) -> crate::bytes::Bytes {
        mapped.bytes()
    }
}

/// As by reference: the mapping moves into the `Bytes`.
impl From<Mapped> for crate::bytes::Bytes {
    fn from(mapped: Mapped) -> crate::bytes::Bytes {
        crate::bytes::Bytes::from_arc(mapped.pages)
    }
}

/// A mapping handed to a parameter that keeps a `Bytes`: at a count.
/// The file it maps and its length, never its bytes: a mapping may be larger
/// than memory, and it is read as the program reads it, not when it is shown.
impl std::fmt::Debug for Mapped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mapped")
            .field("path", &self.pages.asked)
            .field("len", &self.pages.len())
            .finish()
    }
}

impl crate::bytes::KeptBytes for Mapped {
    fn kept_bytes(&self) -> std::borrow::Cow<'_, crate::bytes::Bytes> {
        std::borrow::Cow::Owned(self.bytes())
    }
}

impl Mapped {
    /// **The mapping as a `Bytes`** (ADR-373 D3): the same pages at one more
    /// count, kept mapped while the `Bytes` or a cut of it lives.
    pub fn bytes(&self) -> crate::bytes::Bytes {
        crate::bytes::Bytes::from_arc(self.pages.clone())
    }

    /// **The mapped file, as bytes**: what the ledger's `fs::Mapped::deref`
    /// names, written as a call (#452). The lowering writes `data.deref()`,
    /// and the trait's method is reached only where `Deref` is in scope.
    #[allow(clippy::should_implement_trait)]
    pub fn deref(&self) -> &[u8] {
        &self.pages
    }

    /// **The mapped file, as text** (`fs::Mapped::text`, ADR-016 D5): the
    /// first call checks the whole mapping, in chunks across the pool, and
    /// every later one hands back the same view without checking again.
    pub fn text(&self) -> Result<&str, crate::io::IoError> {
        self.pages.text()
    }
}

/// **A mapped file is read as bytes are** (ADR-016 D5): `data[i]` is a `u8`
/// and `data[a..<b]` a run of them, by the same rule as a `Bytes`'s.
impl<I> crate::index::Get<I> for Mapped
where
    I: std::slice::SliceIndex<[u8]> + 'static,
{
    type Out<'a>
        = &'a I::Output
    where
        Self: 'a;

    #[track_caller]
    fn get(&self, key: I) -> &I::Output {
        &self.deref()[key]
    }
}

impl std::ops::Deref for Mapped {
    type Target = Pages;

    fn deref(&self) -> &Pages {
        &self.pages
    }
}

impl AsRef<[u8]> for Mapped {
    fn as_ref(&self) -> &[u8] {
        self.deref()
    }
}

/// A mapping is a buffer views are cut from, so a view of it kept by a
/// container that drops entries is found in it again (ADR-tether-v1 D15).
impl crate::tether::Viewed for Mapped {
    fn bytes(&self) -> &[u8] {
        self.deref()
    }
}

/// Map a file and make its contents addressable as bytes.
///
/// Fails as the file system does, and only so: whether the bytes are text is
/// asked where they are read as text ([`Mapped::text`], a grammar entered with
/// the mapping), not here (ADR-016 D5).
///
/// **`async` with nothing awaited inside it, and that costs nothing.** A
/// mapping is an `open`, a `stat` and an `mmap` - a path walk and a page-table
/// change, with no data transfer to complete and so nothing to suspend on
/// (`rt::uring`'s own note about what is not completed there). It is `async`
/// because the ledger says it does I/O and may pause, which is a claim about
/// the *surface* - and an `async fn` that never awaits finishes on its first
/// poll ([ADR-055](../../../docs/specification/adr/adr-055.md) D1). A page
/// fault later is not a suspension point this language can see, and pretending
/// otherwise would be a promise nothing keeps.
pub async fn map(path: impl AsRef<FilePath>, root: &Root) -> Result<Mapped, crate::io::IoError> {
    // **The path, held**: an operating system's error does not carry what was
    // asked for, and `NotFound` without it is the round trip to the user
    // [ADR-023](../../../docs/specification/adr/adr-023.md) D3 names. What is
    // *held* is what the caller asked for and what is *used* is what the root
    // resolved it to, so a refusal names the name the program wrote.
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let path = resolved.as_path();
    let named = |e| crate::io::IoError::of(e, asked);
    let file = std::fs::File::open(path).map_err(named)?;
    let map = if file.metadata().map_err(named)?.len() == 0 {
        Backing::Empty
    } else {
        // Mapped in the crate that holds what that takes (ADR-218), which
        // says what a map cannot rule out.
        Backing::Pages(checked_text::OnDemand::new(
            checked_text::map(&file).map_err(named)?,
        ))
    };
    Ok(Mapped {
        pages: std::sync::Arc::new(Pages {
            map,
            asked: asked.to_path_buf(),
        }),
    })
}

/// A whole file, as text.
///
/// Part III, 17.1. The other half of `map`, and the one to reach for when the
/// file is small or has to outlive the parse: this copies the bytes into a
/// `String` the caller owns, where `map` hands back pages it does not.
///
/// Fails as the file system does, and additionally when the file is not
/// UTF-8 - the rule a mapping's `text()` follows, for the same reason.
///
/// **The read goes through the runtime**
/// ([ADR-303](../../../docs/specification/adr/adr-303.md) D3): the kernel
/// completes it where the machine has a completion queue, and an I/O worker
/// performs it where it does not. Which one is invisible from here and
/// invisible from a `.nika` file - that is what "one `std` surface" means, and
/// it is why the next change of mechanism is a `std` change rather than a
/// compiler change (ADR-292 gave the same reason for the overlap
/// vehicle).
///
/// The read is `rt::io`'s rather than [`read`]'s, and deliberately: `read`
/// hands back a shared buffer and this wants the bytes themselves, so going
/// through it would buy a handle only to copy out of it.
pub async fn read_to_string(
    path: impl AsRef<FilePath>,
    root: &Root,
) -> Result<crate::block::Text, crate::io::IoError> {
    // **The name is written out only for a failure** (ADR-263 D3): building
    // it on every call was ~425 instructions a read spent on a message no
    // successful read reports.
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let bytes = crate::rt::io::reading(&resolved)
        .await
        .map_err(|e| crate::io::IoError::of(e, asked))?;
    text_named(bytes, || asked.display().to_string()).map(crate::block::Text::from)
}

/// **A whole file, as text, read into `into`** (#2859): the buffer a loop
/// freed, emptied and filled again, so that a read per turn allocates once.
/// What `read_to_string` does otherwise, and fails as it does; the lowering
/// hands it the buffer whose last view died, and a program may too.
pub async fn read_to_string_into(
    path: impl AsRef<FilePath>,
    root: &Root,
    into: crate::block::Text,
) -> Result<crate::block::Text, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let bytes = crate::rt::io::reading_into(&resolved, String::from(into).into_bytes())
        .await
        .map_err(|e| crate::io::IoError::of(e, asked))?;
    text_named(bytes, || asked.display().to_string()).map(crate::block::Text::from)
}

/// Bytes as text, or the failure `read_to_string` reports for bytes that are
/// not, with the name asked for only when the bytes are not text.
fn text_named(bytes: Vec<u8>, what: impl FnOnce() -> String) -> Result<String, crate::io::IoError> {
    // The same check a mapping's `text()` makes, and the same reason: a parser handed bytes
    // that are not text would find that out one view at a time.
    match checked_text::CheckedText::check(bytes) {
        Ok(text) => Ok(text.into_string()),
        Err(at) => Err(crate::io::IoError::NotText(format!(
            "{} (at byte {at})",
            what()
        ))),
    }
}

/// Two files, **both in flight at once**, answered in the order they were
/// asked for.
///
/// ADR-292's prediction, as a function: two operations that meet on
/// nothing and do not wait for each other, with **no thread started or woken
/// for the pair** - the cost §8.4 measured at ~46 µs for a pair on the pool and
/// could not remove with any user-space vehicle.
///
/// It is available at `user_parallelism = no`, and that is not a loophole: the
/// two reads are `std`'s own operations and nothing the *user* wrote runs
/// concurrently ([ADR-037](../../../docs/specification/adr/adr-037.md) D2,
/// [ADR-016](../../../docs/specification/adr/adr-016.md) D3).
///
/// **It always overlaps**, because a program asked for it - on the completion
/// path for nothing, and on the blocking fallback for the ~38 µs a worker's
/// wake-up costs, which above a quarter megabyte a pair it earns back
/// (ADR-303). There used to be a quieter twin for the pairs the
/// *compiler* put together, which overlapped only where that was free; the
/// automatic grouping is withdrawn ([ADR-292](../../../docs/specification/adr/adr-292.md)
/// D1) and the twin went with it, so this is the only pair vehicle left and
/// every use of it is a written one.
pub fn read_both(
    a: impl AsRef<FilePath>,
    b: impl AsRef<FilePath>,
) -> (
    Result<Vec<u8>, std::io::Error>,
    Result<Vec<u8>, std::io::Error>,
) {
    crate::rt::io::read_both(a.as_ref(), b.as_ref())
}

/// A whole file, as bytes.
///
/// Part III 17.1. The half of `read_to_string` that does not check: an input
/// that is not text is not a failure here, because nothing downstream is going
/// to cut a `&str` out of it. Reach for this where the bytes are the point -
/// an image, a checksum, a format with a length prefix.
///
/// **`Bytes` and not a `Vec[u8]`**, which is what Part III 17.2 has always
/// said and [ADR-tether-v1](../../../docs/specification/adr/adr-tether-v1.md) D21 makes
/// true: one shared buffer, so handing the file on costs a count rather than a
/// copy of it.
pub async fn read(
    path: impl AsRef<FilePath>,
    root: &Root,
) -> Result<crate::bytes::Bytes, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    crate::rt::io::reading(&resolved)
        .await
        .map(crate::bytes::Bytes::from)
        .map_err(|e| crate::io::IoError::of(e, asked))
}

/// A whole file, written.
///
/// Part III 17.1. The file is created if it is not there and **truncated if it
/// is**, which is what `write` means everywhere and is why the specification
/// gives it `create: bool = true` as the default rather than as a decision at
/// the call.
///
/// A whole file, written.
///
/// Part III 17.1, in full:
///
/// ```nika
/// fs::write(path, data)                        // create or truncate
/// fs::write(path, data; append: true)          // add to the end
/// fs::write(path, data; create: false)         // refuse to make a new file
/// ```
///
/// `append` and `create` are Kap 5.1 **options** - after the `;`, named at the
/// call, never positional. The lowering makes them ordinary parameters in
/// declaration order and fills in the defaults a caller left out, so this
/// signature is what a Nikaia call expands to rather than what it looks like.
///
/// The defaults are the ones the specification names, and they are what `write`
/// means everywhere: create the file if it is not there, and truncate it if it
/// is. `append` keeps what is there and adds; `create: false` refuses to make a
/// file that does not exist, which is how a program says it means to overwrite
/// something in particular.
///
/// Takes anything that is bytes, so a Nikaia program hands it a `String`, a
/// `&str` or a buffer without saying which it meant.
///
/// It is not `sync` (Part II, 12.1) and it is not a source (ADR-010 D2): it
/// does I/O, and the bytes travel out of the program rather than in.
pub async fn write(
    path: impl AsRef<FilePath>,
    root: &Root,
    data: impl AsRef<[u8]>,
    append: bool,
    create: bool,
) -> Result<(), crate::io::IoError> {
    // Through the runtime, like the reads (ADR-303 D3). The bytes travel as a
    // borrowed slice and the runtime owns the copy only where the *kernel*
    // needs one to outlive the submission - which is the completion path, and
    // is `rt::uring`'s soundness rule rather than a convenience.
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    crate::rt::io::writing(&resolved, data.as_ref(), append, create)
        .await
        .map(|_| ())
        .map_err(|e| crate::io::IoError::of(e, asked))
}

/// Where `nikaia test` puts a test's own directory, for [`scratch`]
/// ([ADR-247](../../../docs/specification/adr/adr-247.md) D5).
pub const TEST_DIR_VAR: &str = "NIKAIA_TEST_DIR";

/// **A fresh, empty directory, as a [`Root`]**
/// ([ADR-247](../../../docs/specification/adr/adr-247.md) D5): what a test
/// hands a function that writes, so that what it writes is checked to stay
/// inside a place of the test's own (ADR-108 D3).
///
/// Under `nikaia test` it is made inside the test's own directory, which the
/// runner removes when the test's process ends. Anywhere else it is a new
/// directory under the system's temporary directory, left to the operating
/// system as any temporary file is. Each call is a new directory: two tests,
/// or two calls in one, never share one.
pub fn scratch() -> Result<Root, crate::io::IoError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let under = match std::env::var_os(TEST_DIR_VAR) {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::env::temp_dir(),
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let name = format!(
        "nikaia-scratch-{}-{}-{nanos}",
        std::process::id(),
        CALLS.fetch_add(1, Ordering::Relaxed)
    );
    let dir = under.join(name);
    std::fs::create_dir_all(&dir).map_err(|e| crate::io::IoError::of(e, &dir))?;
    Ok(Root::Dir(dir))
}

/// **A file written through a buffer, flushed when it is done with**
/// ([ADR-297](../../../docs/specification/adr/adr-297.md) D11): what `create`
/// hands back, and the first type in `std` whose cleanup needs I/O.
///
/// `write` adds to the buffer and does no I/O; `flush` writes what is held; and
/// a writer nobody flushed is flushed by its cleanup, at the end of the scope
/// that owned it last. That write can fail, so the function that owns one can
/// fail with it — the decades-old `stdio` data-loss bug, told the truth about
/// (D3). `close()` does it at a named moment and hands the failure back
/// as that call's.
pub type Writer = crate::cleanup::CleanedSend<Buffered>;

/// What a [`Writer`] holds: where the bytes go, and those not written yet.
pub struct Buffered {
    path: std::path::PathBuf,
    shown: String,
    held: Vec<u8>,
}

impl Buffered {
    /// Add text to what the file will hold. No I/O.
    pub fn write(&mut self, data: impl AsRef<[u8]>) {
        self.held.extend_from_slice(data.as_ref());
    }

    /// Write what is held to the file, pausing while it is written.
    pub async fn flush(&mut self) -> Result<(), crate::io::IoError> {
        if self.held.is_empty() {
            return Ok(());
        }
        crate::rt::io::writing(&self.path, &self.held, true, true)
            .await
            .map_err(|e| crate::io::IoError::of(e, &self.shown))?;
        self.held.clear();
        Ok(())
    }
}

impl crate::cleanup::CleanupSend for Buffered {
    fn cleanup(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move { self.flush().await.map_err(|e| e.to_string()) })
    }

    fn describe(&self) -> String {
        format!("the file `{}`", self.shown)
    }
}

/// **A file to write, empty**: made where it is not there, emptied where it
/// is. What is written to it is held until it is flushed, closed, or done
/// with.
pub async fn create(path: impl AsRef<FilePath>, root: &Root) -> Result<Writer, crate::io::IoError> {
    let shown = path.as_ref().display().to_string();
    let path = resolve(path.as_ref(), root)?;
    crate::rt::io::writing(&path, b"", false, true)
        .await
        .map_err(|e| crate::io::IoError::of(e, &shown))?;
    Ok(crate::cleanup::CleanedSend::new(Buffered {
        path,
        shown,
        held: Vec::new(),
    }))
}

/// **A file held open** (Part III 17.1, #546): read, written and moved about
/// in, and flushed and closed when the scope that owns it last ends - or at a
/// named moment with `close()`, which hands the failure back as that call's
/// ([ADR-297](../../../docs/specification/adr/adr-297.md) D4).
pub type File = crate::cleanup::CleanedSend<Opened>;

/// Where `File::seek` moves to: a byte position from the start, or a distance
/// from where it is or from the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seek {
    Start(i64),
    Current(i64),
    End(i64),
}

/// What a [`File`] holds: the open file and its name, for a refusal.
///
/// **Every call is the system's own, without a buffer and not through the I/O
/// reactor**, as [`exists`] is: a program that wants its bytes gathered writes
/// them at once, or uses [`create`]'s writer.
pub struct Opened {
    file: std::fs::File,
    shown: String,
}

/// What a struct that holds a [`File`] prints for `Debug` (#2238): its name.
impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fs::File({})", self.shown)
    }
}

impl Opened {
    fn failed(&self, error: std::io::Error) -> crate::io::IoError {
        crate::io::IoError::of(error, &self.shown)
    }

    /// Bytes into `into`, from where the file is; how many, `0` at its end.
    pub fn read(&mut self, into: &mut [u8]) -> Result<i64, crate::io::IoError> {
        use std::io::Read;
        let n = self.file.read(into).map_err(|e| self.failed(e))?;
        Ok(i64::try_from(n).unwrap_or(i64::MAX))
    }

    /// Bytes, or text, into the file where it is; how many were written.
    pub fn write(&mut self, data: impl AsRef<[u8]>) -> Result<i64, crate::io::IoError> {
        use std::io::Write;
        let data = data.as_ref();
        self.file.write_all(data).map_err(|e| self.failed(e))?;
        Ok(i64::try_from(data.len()).unwrap_or(i64::MAX))
    }

    /// What was written, handed to the system.
    pub fn flush(&mut self) -> Result<(), crate::io::IoError> {
        use std::io::Write;
        self.file.flush().map_err(|e| self.failed(e))
    }

    /// Moves to `to`; the byte position it is at now. A position before the
    /// start is refused.
    pub fn seek(&mut self, to: Seek) -> Result<i64, crate::io::IoError> {
        use std::io::Seek as _;
        let to = match to {
            Seek::Start(at) => std::io::SeekFrom::Start(u64::try_from(at).map_err(|_| {
                crate::io::IoError::Other(format!("{}: no position {at}", self.shown))
            })?),
            Seek::Current(by) => std::io::SeekFrom::Current(by),
            Seek::End(by) => std::io::SeekFrom::End(by),
        };
        let at = self.file.seek(to).map_err(|e| self.failed(e))?;
        Ok(i64::try_from(at).unwrap_or(i64::MAX))
    }

    /// How many bytes the file holds now, an `i64` as a length is.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> Result<i64, crate::io::IoError> {
        let known = self.file.metadata().map_err(|e| self.failed(e))?;
        Ok(i64::try_from(known.len()).unwrap_or(i64::MAX))
    }
}

impl crate::cleanup::CleanupSend for Opened {
    fn cleanup(
        &mut self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send + '_>> {
        Box::pin(async move { self.flush().map_err(|e| e.to_string()) })
    }

    fn describe(&self) -> String {
        format!("the file `{}`", self.shown)
    }
}

/// **A file, opened** to read, and to write where `write` or `append` says so;
/// `create` makes it where it is not there, `truncate` empties it.
pub fn open(
    path: impl AsRef<FilePath>,
    root: &Root,
    write: bool,
    append: bool,
    create: bool,
    truncate: bool,
) -> Result<File, crate::io::IoError> {
    let asked = path.as_ref();
    let resolved = resolve(asked, root)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(write)
        .append(append)
        .create(create)
        .truncate(truncate)
        .open(&resolved)
        .map_err(|e| crate::io::IoError::of(e, asked))?;
    Ok(crate::cleanup::CleanedSend::new(Opened {
        file,
        shown: asked.display().to_string(),
    }))
}

/// **Every file under a directory**, as names relative to it, `/` between the
/// parts, in sorted order ([ADR-rust-boundary-1](../../../docs/specification/adr/adr-rust-boundary-1.md)
/// D4).
///
/// What a tool that reads a tree needs, and the shape `nikaia describe` reads a
/// crate's sources in: the files themselves, recursively, with the directories
/// left out, because a directory is never what a caller then reads. Sorted, so
/// two runs over the same tree hand back the same list — an order the operating
/// system happens to keep is not one a program may rest on.
///
/// **The root holds for every name the walk finds, and not only for the one it
/// was asked for** ([ADR-108](../../../docs/specification/adr/adr-108.md) D3).
/// A symlink inside the directory may point out of it; under `Root::Dir` such an
/// entry is *not listed*, rather than failing the walk, because the program did
/// not ask for that name and a tree with one stray link in it is still a tree
/// worth reading. `Root::Anywhere` follows it, as it follows everything.
///
/// **A directory is visited once**, by its resolved name, so a link that points
/// back up the tree ends the walk instead of looping it.
///
/// It fails where the directory itself cannot be read. Each name is an `fs::Path`
/// relative to the directory asked for, in the platform's own bytes (ADR-319
/// D7): one that is not text is listed as it is, and opens the same file.
///
/// `async` with nothing awaited, for the reason [`map`] gives: the ledger says
/// it does I/O and may pause.
pub async fn walk(
    path: impl AsRef<FilePath>,
    root: &Root,
) -> Result<crate::block::List<Path>, crate::io::IoError> {
    let start = resolve(path.as_ref(), root)?;
    let base = match root {
        Root::Dir(dir) => Some(
            std::fs::canonicalize(FilePath::new(dir))
                .map_err(|_| crate::io::IoError::Outside(path.as_ref().to_path_buf()))?,
        ),
        Root::Anywhere => None,
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut found = Vec::new();
    // The directory asked for has to be readable; one under it that is not is
    // left out, as a name outside the root is.
    std::fs::read_dir(&start).map_err(|e| crate::io::IoError::of(e, path.as_ref()))?;
    let mut pending = vec![(start, Path::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let Ok(real) = std::fs::canonicalize(&dir) else {
            continue;
        };
        if !seen.insert(real) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        // **A name is listed as the platform's bytes** (ADR-319 D7): one
        // that is not text is a name like any other, and `to_text` is where
        // a program asks whether it is text.
        for entry in entries.flatten() {
            let name = entry.file_name();
            let at = entry.path();
            let Ok(real) = std::fs::canonicalize(&at) else {
                continue;
            };
            if base.as_ref().is_some_and(|base| !real.starts_with(base)) {
                continue;
            }
            let relative = prefix.join(&name);
            match real.is_dir() {
                true => pending.push((at, relative)),
                false => found.push(relative),
            }
        }
    }
    found.sort();
    Ok(crate::block::List::from(found))
}
