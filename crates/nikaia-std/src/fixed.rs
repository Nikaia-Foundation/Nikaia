//! A map whose keys the build knew
//! ([ADR-311](../../../docs/specification/adr/adr-311.md) D9,
//! [ADR-311](../../../docs/specification/adr/adr-311.md) D6).
//!
//! **Four tables and no generated code.** The obvious lowering for a small map
//! was a `match` written out by the emitter, because that is what the numbers
//! favour against a hash. What [`docs/history/fixed-map-lookup.md`](../../../docs/history/fixed-map-lookup.md)
//! §6 then measured is a **linear scan over these very arrays**, and it is the
//! `match` within 0 to 17 % up to twenty-four keys — a dozen past the point
//! where the perfect hash has already taken the lead. So the range where a
//! `match` would be worth generating is empty, and what a `comptime` crosses as
//! is a value rather than a function.
//!
//! Which buys three things. A `Fixed` is a **value** a program may pass and
//! store. The emitter stays one that knows no types
//! ([ADR-296](../../../docs/specification/adr/adr-296.md) D17), because there is
//! nothing to write but a `const`. And the threshold lives in the data — a
//! table with no displacements is a small one — rather than as a branch in the
//! compiler.

/// A map built while the program was built.
///
/// Everything in it is `&'static`, which is [ADR-311](../../../docs/specification/adr/adr-311.md)
/// D1's *growable going in, fixed coming out*: what the build owned, the program
/// gets a view of.
#[derive(Debug, Clone, Copy)]
pub struct Fixed<V: 'static> {
    /// The seed the displacements were found with. Unused where there are none.
    seed: u64,
    /// Empty for a small table, which is read by walking it.
    disps: &'static [(u32, u32)],
    /// In slot order where there are displacements, and in written order where
    /// there are not.
    keys: &'static [&'static str],
    vals: &'static [V],
}

impl<V> Fixed<V> {
    /// **Written by the compiler and by nothing else.** A `Fixed` is what a
    /// `comptime` crosses as, and the tables are only consistent because a
    /// generator made them together.
    pub const fn new(
        seed: u64,
        disps: &'static [(u32, u32)],
        keys: &'static [&'static str],
        vals: &'static [V],
    ) -> Self {
        Fixed {
            seed,
            disps,
            keys,
            vals,
        }
    }

    /// What this key names, or nothing where the key is not there.
    ///
    /// **Two shapes under one name**, and which one a table is was decided while
    /// the program was built: no displacements means few enough keys that
    /// walking them beats hashing one.
    ///
    /// **A value and not a view of one**, which is what the ledger promises
    /// (`-> $V?`) and what a reader of `ROUTES.get(k) ?? 0` means. `Option<&V>`
    /// would be the borrow-free thing to write and it put a `&&str` in the
    /// generated file for a table of text, where `??` is ambiguous between two
    /// `Or` impls — `rustc` complaining about a file nobody wrote, which
    /// [Part III C.1](../../../docs/specification/30-nikaia-tooling.md) does not
    /// allow. Everything a `const` of this kind holds is `Copy`, so the bound
    /// costs nothing: the compiler will not write a table whose values are not.
    pub fn get(&self, key: &str) -> Option<V>
    where
        V: Copy,
    {
        let at = match self.disps.is_empty() {
            // **The length first**, which is what makes a scan competitive: most
            // keys are ruled out without touching their bytes.
            true => self
                .keys
                .iter()
                .position(|held| held.len() == key.len() && *held == key)?,
            false => {
                let hash = crate::runtime::fnv(key, self.seed);
                let (g, f1, f2) = ((hash >> 32) as u32, hash as u32, (hash >> 16) as u32);
                let (d1, d2) = self.disps[(g as usize) % self.disps.len()];
                let at = (crate::runtime::displace(f1, f2, d1, d2) as usize) % self.keys.len();
                // **A perfect hash still compares.** It says *if this key is in
                // the table it is in this slot*, which is not the same as
                // saying it is there.
                if self.keys[at] != key {
                    return None;
                }
                at
            }
        };
        self.vals.get(at).copied()
    }

    /// Whether this key is one of the table's.
    pub fn has(&self, key: &str) -> bool
    where
        V: Copy,
    {
        self.get(key).is_some()
    }

    /// How many pairs the table holds.
    pub fn len(&self) -> i64 {
        self.keys.len() as i64
    }

    /// Whether the table holds nothing. Beside `len`, because Rust's own lint
    /// asks for the pair and a reader expects it.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}
