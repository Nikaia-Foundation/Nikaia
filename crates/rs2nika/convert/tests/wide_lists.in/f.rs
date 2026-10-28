const XS: &[&str] = &[
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "x86_64-pc-windows-msvc",
];
fn f() -> Vec<i64> {
    let v = vec![
        1,
        2,
    ];
    let b = Build {
        target: parse_target(target_name_given_on_the_command_line),
        overflow: overflow_checks_of(the_build_settings),
    };
    v
}
