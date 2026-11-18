; A symbol may hold a `.` (SMT-LIB 2.6 3.1); the reader once took it for a decimal.
(set-logic QF_LIA)
(set-info :status sat)
(declare-fun main_~i.4 () Int)
(assert (>= main_~i.4 1))
(check-sat)
