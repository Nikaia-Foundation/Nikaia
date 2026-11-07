; An uninterpreted function: outside what the reader takes.
(set-logic QF_UFLIA)
(set-info :status sat)
(declare-fun f (Int) Int)
(assert (= (f 1) 2))
(check-sat)
