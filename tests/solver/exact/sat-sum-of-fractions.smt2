; Exact sums (#2013): the cross products of 1/9111001479 + 1/9111001485 leave
; i64, the reduced sum 2024666996/9223372000556244035 fits; the sum is
; added in double width (narrowing_div_rem) instead of giving up.
(set-info :status sat)
(set-logic QF_LRA)
(declare-fun x () Real)
(assert (= x (+ (/ 1 9111001479) (/ 1 9111001485))))
(assert (>= x (/ 2024666996 9223372000556244035)))
(check-sat)
(exit)
