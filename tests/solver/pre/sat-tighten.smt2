; Preprocessing (#1283 item 5): `x <= 4` decides both alternatives of the
; first disjunction at level zero; `3y >= 7` comes as `y >= 3` from the gcd
; normal form of the atoms.
(set-logic QF_LIA)
(set-info :status sat)
(declare-const x Int)
(declare-const y Int)
(assert (<= x 4))
(assert (or (<= x 7) (>= x 9)))
(assert (or (>= (* 3 y) 7) (<= y 0)))
(assert (or (>= (+ x y) 5) (<= (- x y) 2)))
(check-sat)
