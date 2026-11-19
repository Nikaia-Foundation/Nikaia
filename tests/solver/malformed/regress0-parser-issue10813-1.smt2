; From cvc5's regress0/parser/issue10813-1.smt2, a parser test whose
; expected output is the error "cannot construct Real or Int from string
; argument". Ill-formed: the literal `5/0` has a zero denominator (z3:
; unknown constant /0).
; EXPECT: error
(set-logic QF_LRA)
(declare-const x Real)
(declare-const y Real)
(assert (> x 1.5))
(assert (< y 3.5))
(assert (= (+ x y) 5/0))
(check-sat)
