Result: done
Batch: y10-1030-literal-label.md y2c-properties.md y2b-keys-directives-limits.md
Owner: none
Cut: y9 done (9293d338, #1027); its Found filed as #1030 (label `ready`): a label on a literal returned or stored as `String` reaches rustc; cause traced to `CheckState::pattern_type` lacking a `Pattern::Literal` arm. Fix brief y10 first (small, nothing in yaml needs it); y2c (properties; draft captures with `text(..)`, checked) and y2b follow unchanged except y2c's pointer to y10.
Changed: #928 To build (y9 struck, y10 added), #1030 filed; docs/builder-facts.md: two y9 traps (grammar rustc errors: check+lower first; tests/compiler past 10 min needs timeout 3600000).
Decided for the supervisor: y10 adds a sentence to Part II 10.8 (`"text"` row: a label on it binds the text matched), so it raises the version.
