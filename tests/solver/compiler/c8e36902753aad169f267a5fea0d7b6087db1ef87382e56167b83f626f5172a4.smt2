(set-logic QF_LIA)
(set-info :status unsat)
(assert (not (= (- (+ 0 32) 0) (- (+ 1 32) 1))))
(check-sat)
