; From pysmt issue 159. Ill-formed: a parallel `let` binds its names only in
; its body, so `a` in the binding of `b` is unbound (z3: unknown constant a).
; EXPECT: error
(set-logic QF_LIA)
(declare-fun x () Int)
(assert (let ((a x)
              (b (+ a x)))
          (> b a)))
(check-sat)
