(set-logic QF_LIA)
(set-info :status unsat)
(assert (not (= (- (+ 0 4) 0) (- (+ 1 4) 1))))
(check-sat)
