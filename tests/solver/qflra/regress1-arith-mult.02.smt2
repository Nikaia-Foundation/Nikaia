; EXPECT: (error "A non-linear term was asserted to arithmetic in a linear logic.")
; EXIT: 1
(set-logic QF_LRA)
(set-info :status sat)
(declare-fun n () Real)

; This example is test that LRA rejects multiplication terms

(assert (= (* n n) 1))

(check-sat)
