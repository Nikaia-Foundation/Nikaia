(set-logic QF_LIA)
(set-info :status sat)
(assert (not (= (- (* 0 10) 0) (- (* 1 10) 1))))
(check-sat)
