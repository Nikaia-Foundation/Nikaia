; Preprocessing: the two `(>= x 5)` of the `or` give two unit clauses of
; one literal; the refutation hints it once (#1957, ERK-PT-000100-RF-00).
(set-logic QF_LIA)
(set-info :status unsat)
(declare-const x Int)
(declare-const y Int)
(declare-const z Int)
(assert (= z (+ x y)))
(assert (= z 10))
(assert (<= y 5))
(assert (not (or (>= x 5) (>= x 5))))
(check-sat)
