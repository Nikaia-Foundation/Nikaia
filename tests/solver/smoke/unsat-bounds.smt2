; The bounds check of ADR-270 D9: 0 <= i < n proves i < n + 1.
(set-logic QF_LIA)
(set-info :status unsat)
(declare-const i Int)
(declare-const n Int)
(assert (<= 0 i))
(assert (< i n))
(assert (not (< i (+ n 1))))
(check-sat)
