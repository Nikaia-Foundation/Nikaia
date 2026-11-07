; Preprocessing (#1283 item 5): the equality's gcd 2 does not divide 3, so
; the equality elimination refutes it before the loop.
(set-logic QF_LIA)
(set-info :status unsat)
(declare-const x Int)
(declare-const y Int)
(assert (= (+ (* 2 x) (* 4 y)) 3))
(check-sat)
