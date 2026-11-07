; A model: 1 <= x <= 3 and x != 2.
(set-logic QF_LIA)
(set-info :status sat)
(declare-const x Int)
(assert (<= 1 x))
(assert (<= x 3))
(assert (distinct x 2))
(check-sat)
