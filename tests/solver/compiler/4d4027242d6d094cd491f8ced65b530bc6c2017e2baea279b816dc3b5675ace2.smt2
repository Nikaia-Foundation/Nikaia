(set-logic QF_LIA)
(set-info :status unsat)
(assert (not (= (- (+ 0 2) 0) (- (+ 1 2) 1))))
(check-sat)
