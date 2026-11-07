(set-logic QF_LIA)
(set-info :status sat)
(assert (not (= (- (+ (* 2 0) 1) 0) (- (+ (* 2 1) 1) 1))))
(check-sat)
