(set-logic QF_LIA)
(set-info :status unsat)
(assert (not (= (- (+ 0 1) 0) (- (+ 1 1) 1))))
(check-sat)
