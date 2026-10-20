# Sourced by `fetch.sh` and `humans.sh`: builds one binary of a human-written
# 1BRC program at every setting the table has. Each setting gets a target
# directory of its own under `benches/brc/target/human/build`, and the binary
# is copied out to `benches/brc/target/human/bin/NAME.SETTING`.
#
# The bench's settings are `brc.sh`'s for its Rust programs: `opt-level = 3`,
# Cargo's default `lto = false` (no link-time optimisation across crates),
# `panic = "unwind"`, codegen units 1 and 16, the default CPU level `x86-64`,
# and the same at `x86-64-v3`. The authors' projects say otherwise -
# `target-cpu=native` in `.cargo/config.toml`, fat LTO, one unit and
# `panic = "abort"` in `[profile.release]` - and they are overridden here
# without editing them: the profile through `CARGO_PROFILE_RELEASE_*`, the
# CPU through `RUSTFLAGS`, which Cargo takes instead of the project's
# `build.rustflags` whenever it is set, empty at the default level (and
# `--config 'build.rustflags=[]'`, a command-line `--config` that wins over
# the project's file, says the same).
# Symbols are kept (`strip = "none"`), which changes no instruction and lets
# callgrind name the functions.
#
# Two settings build the project as it is: `authors` exactly as its authors
# do (`cargo build --release`, `native` included), and `authors-v3` with their
# profile and the CPU capped at `x86-64-v3`, the one of the two that callgrind
# can run on a CPU with AVX-512.

human_out="$root/benches/brc/target/human"
human_settings="cu1 cu16 v3-cu1 v3-cu16 authors authors-v3"

# human_build DIR NAME BIN: builds `--bin BIN` of the Cargo project in DIR at
# every setting, as `bin/NAME.SETTING`.
human_build() {
    local dir=$1 name=$2 bin=$3 s u
    mkdir -p "$human_out/bin"
    for s in $human_settings; do
        local tdir="$human_out/build/$s"
        case $s in
        authors)
            ( cd "$dir" && env -u RUSTFLAGS CARGO_TARGET_DIR="$tdir" \
                cargo build --release --quiet --bin "$bin" ) ;;
        authors-v3)
            ( cd "$dir" && RUSTFLAGS="-C target-cpu=x86-64-v3" CARGO_TARGET_DIR="$tdir" \
                cargo build --release --quiet --bin "$bin" ) ;;
        *)
            u=${s##*cu}
            local flags=""
            case $s in v3-*) flags="-C target-cpu=x86-64-v3" ;; esac
            ( cd "$dir" && RUSTFLAGS="$flags" \
                CARGO_TARGET_DIR="$tdir" \
                CARGO_PROFILE_RELEASE_OPT_LEVEL=3 CARGO_PROFILE_RELEASE_LTO=false \
                CARGO_PROFILE_RELEASE_CODEGEN_UNITS="$u" CARGO_PROFILE_RELEASE_PANIC=unwind \
                CARGO_PROFILE_RELEASE_STRIP=none CARGO_PROFILE_RELEASE_DEBUG=false \
                cargo build --release --quiet --bin "$bin" --config 'build.rustflags=[]' ) ;;
        esac || { echo "build failed: $name at $s" >&2; return 1; }
        cp "$tdir/release/$bin" "$human_out/bin/$name.$s"
    done
}
