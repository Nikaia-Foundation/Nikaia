; Preprocessing (#1283 item 5): `x >= 5` contradicts both alternatives, the
; second only once `2x <= 3` is rounded to `x <= 1`.
(set-logic QF_LIA)
(set-info :status unsat)
(declare-const x Int)
(declare-const y Int)
(assert (>= x 5))
(assert (or (<= x 2) (<= (* 2 x) 3)))
(assert (or (>= y 0) (<= y (- 3))))
(check-sat)
