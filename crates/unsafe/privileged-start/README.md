# privileged-start

Whether this process was **started with more rights than its caller**: a
setuid or setgid program, or one with file capabilities
([ADR-397](../../../docs/specification/adr/adr-397.md) D3). One function,
`privileged() -> bool`. A Nikaia program whose build takes the environment as
the operator's (`[build] environment = "operator"`, the default) asks it before
`main` and ends with exit status 1 on `true`, since its caller chose every
variable it reads; `nikaia_std::env::stop_a_privileged_start` makes the call.

| platform | how it asks |
| :--- | :--- |
| Linux, Android | `getauxval(AT_SECURE) != 0` |
| macOS, FreeBSD, OpenBSD, NetBSD | `issetugid() != 0` |
| elsewhere, Windows among them | no such start: `false` |

Both functions are declared with `extern "C"` here, as the C library exports
them (glibc, musl, bionic; the BSDs' and Apple's libc), so the crate depends on
nothing, not even `libc`.

## Every `unsafe`, and why it is sound

| where | what | why it holds |
| :--- | :--- | :--- |
| `imp::privileged` (Linux, Android) | the call `getauxval(AT_SECURE)` | the declaration matches the C library's, `unsigned long getauxval(unsigned long)`; it takes an integer and hands back an integer, touches no memory of the caller's, reads the auxiliary vector the kernel wrote before the process started and nobody changes after, and answers 0 for an entry it does not hold, so no argument makes it undefined |
| `imp::privileged` (macOS, the BSDs) | the call `issetugid()` | the declaration matches the C library's, `int issetugid(void)`; it takes nothing, hands back an integer and reads a flag the kernel keeps for the process |

The `unsafe extern "C"` blocks themselves claim only those two signatures,
which the rows above check against the C library's headers.

## How it is checked

```sh
cargo test
cargo clippy --all-targets -- -D warnings
```

The test starts the process the plain way and expects `false`. **Miri cannot
run it**, since it has neither foreign function (the test is ignored under
Miri), and it could not judge these calls anyway: both are integer in,
integer out, with no pointer for Miri to follow. What upholds them is the
signature, checked above.

A privileged start cannot be made inside a test or on CI. It was checked by
hand on Linux (#2239): a program built from this crate, owned by `nobody` with
the setuid bit (`chown nobody` + `chmod u+s`) and started by root, answers
`true`; the same file without the bit answers `false`. A Nikaia program built
with `[build] environment = "operator"` ends with the line and status 1 when
started setuid so, or with a file capability (`setcap cap_net_bind_service+ep`,
started by `nobody`); built with `environment = "untrusted"`, it reaches `main`.
