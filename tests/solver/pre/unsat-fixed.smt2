; Preprocessing (#1283 item 5): `x = 3` is removed from both alternatives,
; which then bound `y` alone.
(set-logic QF_LIA)
(set-info :status unsat)
(declare-const x Int)
(declare-const y Int)
(assert (= x 3))
(assert (<= y 5))
(assert (or (>= (+ x y) 10) (>= (- y x) 10)))
(check-sat)
