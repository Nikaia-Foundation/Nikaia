/// `error` or `warn`, what `--refuted-claims` and `[build] refuted-claims`
/// may say ([ADR-269](../../docs/specification/adr/adr-269.md) D8).
fn refuted_claims(word: &str) -> Result<String> {
    match word {
        "error" | "warn" => Ok(word.to_string()),
        other => Err(anyhow!(
            "`refuted-claims` is `error` or `warn`, not `{other}`. `error` refuses a program \
             with a claim the compiler shows false with values; `warn` builds it and checks the \
             claim when it runs."
        )),
    }
}

/// `error` or `warn`, what `--unproved-claims` and `[build] unproved-claims`
/// may say ([ADR-380](../../docs/specification/adr/adr-380.md) D2).
fn unproved_claims(word: &str) -> Result<String> {
    match word {
        "error" | "warn" => Ok(word.to_string()),
        other => Err(anyhow!(
            "`unproved-claims` is `error` or `warn`, not `{other}`. `error` refuses a program \
             with a claim the compiler did not prove; `warn` builds it and checks the claim \
             when it runs."
        )),
    }
}

/// **`name:on` or `name:off` words, comma-separated**, to what they decide
/// ([ADR-306](../../docs/specification/adr/adr-306.md) D14). The names are
/// `remove-bounds-checks` and `remove-overflow-checks`
/// ([ADR-306](../../docs/specification/adr/adr-306.md) D6); a later word for
/// a name wins over an earlier one.
pub fn optimizations(
    words: &str,
) -> Result<(crate::bounds::BoundsChecks, crate::bounds::OverflowChecks)> {
    let mut bounds = crate::bounds::BoundsChecks::Kept;
    let mut overflow = crate::bounds::OverflowChecks::Kept;
    for word in words.split(',').map(str::trim).filter(|w| !w.is_empty()) {
        let Some((name, level)) = word.split_once(':') else {
            return Err(anyhow!(
                "`--optimization={word}` needs `on` or `off`: write \
                 `--optimization=remove-bounds-checks:on` or \
                 `--optimization=remove-overflow-checks:on`"
            ));
        };
        match name {
            "remove-bounds-checks" => {
                bounds = crate::bounds::BoundsChecks::parse(level).ok_or_else(|| {
                    anyhow!(
                        "`remove-bounds-checks` is `on` or `off`, not `{level}`{}",
                        renamed(level)
                    )
                })?;
            }
            // Read by `skip_free_at_exit` and `prove_through_loops`; checked
            // here with the rest.
            "skip-free-at-exit" | "prove-through-loops" => {
                if !matches!(level, "on" | "off") {
                    return Err(anyhow!("`{name}` is `on` or `off`, not `{level}`"));
                }
            }
            "remove-overflow-checks" => {
                overflow = crate::bounds::OverflowChecks::parse(level).ok_or_else(|| {
                    anyhow!(
                        "`remove-overflow-checks` is `on` or `off`, not `{level}`{}",
                        renamed(level)
                    )
                })?;
            }
            _ => {
                return Err(anyhow!(
                    "there is no optimization called `{name}`: the ones there are, are \
                     `remove-bounds-checks`, `remove-overflow-checks`, `prove-through-loops` \
                     and `skip-free-at-exit`"
                ));
            }
        }
    }
    Ok((bounds, overflow))
}

/// **`prove-through-loops:on`**
/// ([ADR-361](../../docs/specification/adr/adr-361.md) D10): an index's, an
/// overflow's or a shift's check a loop leaves open is asked once more through
/// the loop's invariant. The last word for the name wins; [`optimizations`]
/// has already refused a word that is not `on` or `off`.
pub fn prove_through_loops(words: &str) -> bool {
    words
        .split(',')
        .map(str::trim)
        .filter_map(|w| w.strip_prefix("prove-through-loops:"))
        .next_back()
        .is_some_and(|level| level == "on")
}

/// **`skip-free-at-exit:on`**
/// ([ADR-348](../../docs/specification/adr/adr-348.md) D1): what `main` still
/// holds at its end dies, and every cleanup runs, but its memory is left to the
/// operating system. The last word for the name wins; [`optimizations`] has
/// already refused a word that is not `on` or `off`.
pub fn skip_free_at_exit(words: &str) -> bool {
    words
        .split(',')
        .map(str::trim)
        .filter_map(|w| w.strip_prefix("skip-free-at-exit:"))
        .next_back()
        .is_some_and(|level| level == "on")
}

/// **The words before ADR-306 D14**: `basic` and `aggressive` were levels of
/// how hard the compiler proved and whether it dropped what it proved, in one
/// word. What is proved no longer depends on the option, so the old words name
/// the one thing they still mean.
fn renamed(level: &str) -> &'static str {
    match level {
        "aggressive" | "basic" => {
            ". The compiler now proves the same at every build, and the option only says \
             whether a proved check is written without it: `aggressive` and `basic` are `on`"
        }
        _ => "",
    }
}

/// `[build] artifact` (ADR-284 D4): a program, or a library C calls.
fn artifact(word: &str) -> Result<String> {
    match word {
        "program" | "c-library" => Ok(word.to_string()),
        other => Err(anyhow!(
            "`artifact` is `program` or `c-library`, not `{other}`. `c-library` makes a shared and \
             a static library of the package, and its header, from every `pub extern fn` with a body."
        )),
    }
}
    /// What Cargo is told for the CPU level: `-C target-cpu` for every crate
    /// built for the target, the program's and `std`'s, through Cargo's own
    /// configuration rather than the user's `RUSTFLAGS` (ADR-002 D6, ADR-325).
    pub fn cpu_config(&self) -> Vec<String> {
        match &self.cpu {
            None => Vec::new(),
            Some(level) => vec![format!(
                "target.\"{}\".rustflags=[\"-C\", \"target-cpu={level}\"]",
                self.build.target.triple()
            )],
        }
    }
    /// The cache's view of this build (ADR-021 D5).
    ///
    /// The backend is `"rust"` because it names **the lowering that produced the
    /// artifact**, not the flag the user typed, and a bare invocation *is*
    /// `--backend rust` ([ADR-004](../../docs/specification/adr/adr-004.md) D1),
    /// so the two spellings share one entry - which is correct, because it is
    /// one lowering. The dimension itself stays in
    /// [`orchestrator::cache::Key::build`] (D5, D7) so that the day a second
    /// backend caches, the literal here becomes that backend's name and the two
    /// cannot collide.
    pub fn choices(&self) -> Choices {
        Choices::new(
            format!(
                "{}/{}/{}{}{}{}{}{}{}{}{}{}",
                self.target,
                self.user_parallelism,
                self.reentrancy_check,
                if self.tests { "/tests" } else { "" },
                // A build that lets a refuted claim through must not hand its
                // lowering to one that refuses it.
                if self.refuted_claims == "warn" {
                    "/refuted-claims:warn"
                } else {
                    ""
                },
                // The same for a claim not proved (ADR-380 D2).
                if self.unproved_claims == "warn" {
                    "/unproved-claims:warn"
                } else {
                    ""
                },
                match self.build.bounds {
                    crate::bounds::BoundsChecks::Kept => String::new(),
                    level => format!("/remove-bounds-checks:{}", level.name()),
                },
                match self.build.overflow {
                    crate::bounds::OverflowChecks::Kept => String::new(),
                    level => format!("/remove-overflow-checks:{}", level.name()),
                },
                match self.build.skip_free_at_exit {
                    true => "/skip-free-at-exit:on",
                    false => "",
                },
                match self.build.prove_through_loops {
                    true => "/prove-through-loops:on",
                    false => "",
                },
                // A library's lowering carries its entry points' wrappers,
                // named by the prefix.
                match self.library() {
                    true => format!("/c-library:{}", self.symbol_prefix),
                    false => String::new(),
                },
                // A `comptime` that reads `target::has` answers for the level
                // (ADR-371 D6), so the lowering is kept per level.
                match self.cpu {
                    Some(_) => format!("/cpu:{}", self.cpu_key().unwrap_or_default()),
                    None => String::new(),
                }
            ),
            "rust",
        )
    }
    /// What a panic does on this machine (ADR-037 D1), in Cargo's vocabulary.
    ///
    /// Not a choice, which is why it is not in the codegen table: WebAssembly
    /// traps and has no stack to unwind, and a build that asked for `unwind`
    /// there would be asking for something the machine does not have.
    fn panic_strategy(&self) -> &'static str {
        match self.build.target {
            Target::X86_64Linux | Target::Aarch64Linux => "unwind",
            Target::Wasm32Unknown => "abort",
            target @ Target::Rust(_) => {
                if target.is_bare_metal() || target.triple().starts_with("wasm") {
                    "abort"
                } else {
                    "unwind"
                }
            }
        }
    }
