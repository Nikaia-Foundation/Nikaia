//! Issue #876's workloads in Rust; the `native` variants use x86-64 instructions
//! (`ADC`, `DIV`, `PCLMULQDQ`), so the workloads live in `x86.rs` and other
//! targets build a stub that says so.

#[cfg(target_arch = "x86_64")]
mod x86;

#[cfg(target_arch = "x86_64")]
fn main() {
    x86::run();
}

#[cfg(not(target_arch = "x86_64"))]
fn main() {
    eprintln!("wide-ops: the native variants need x86-64");
    std::process::exit(2)
}
