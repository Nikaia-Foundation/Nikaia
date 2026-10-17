//! **A loop as Horn clauses, and an invariant checked by three queries**
//! (ADR-361 D1, D4; issue #640 steps 1 and 2): the logic layer's loop
//! problem, its SMT-LIB `HORN` text, and what makes a candidate invariant
//! usable.

use nikaia_std::tools::logic_horn::{
    LgExit, LgLoop, lg_horn_write, lg_loop_check, lg_primed, lg_proved,
};
use nikaia_std::tools::prover_arena::TermArena;

/// ADR-361 §1's loop: `while i < a { i = i + 1; total = total + 10 }` from
/// `i == 0`, `total == 0`, with `a > 0` and `a <= 10` holding throughout, and
/// `total == 10 * a` asked past it.
fn section_one(arena: &mut TermArena) -> LgLoop {
    let i = arena.var("i");
    let total = arena.var("total");
    let a = arena.var("a");
    let i1 = arena.var(&lg_primed("i"));
    let total1 = arena.var(&lg_primed("total"));
    let zero = arena.int(0);
    let one = arena.int(1);
    let ten = arena.int(10);
    let a_positive = arena.gt(a, zero);
    let a_small = arena.le(a, ten);
    let i_zero = arena.equal(i, zero);
    let total_zero = arena.equal(total, zero);
    let condition = arena.lt(i, a);
    let i_step = arena.add(i, one);
    let i_turn = arena.equal(i1, i_step);
    let total_step = arena.add(total, ten);
    let total_turn = arena.equal(total1, total_step);
    let step = arena.and(vec![i_turn, total_turn]);
    let not_condition = arena.not(condition);
    let i_same = arena.equal(i1, i);
    let total_same = arena.equal(total1, total);
    let leave = arena.and(vec![not_condition, i_same, total_same]);
    let ten_a = arena.mul(ten, a);
    let goal = arena.equal(total, ten_a);
    LgLoop {
        state: vec!["i".to_string(), "total".to_string()],
        fixed: vec![a_positive, a_small],
        entry: vec![i_zero, total_zero],
        condition: Some(condition),
        step,
        exits: vec![LgExit {
            kind: "condition".to_string(),
            relation: leave,
        }],
        goals: vec![goal],
    }
}

#[test]
fn the_loop_of_section_one_in_horn_clauses() {
    let mut arena = TermArena::empty();
    let problem = section_one(&mut arena);
    let text = lg_horn_write(&mut arena, &problem);
    assert_eq!(
        text,
        "(set-logic HORN)\n\
         (declare-fun inv (Int Int Int) Bool)\n\
         (assert (forall ((a Int) (i Int) (total Int)) (=> (and (> a 0) (<= a 10) (= i 0) (= total 0)) (inv a i total))))\n\
         (assert (forall ((a Int) (i Int) (total Int) (|i'| Int) (|total'| Int)) (=> (and (inv a i total) (> a 0) (<= a 10) (< i a) (and (= |i'| (+ i 1)) (= |total'| (+ total 10)))) (inv a |i'| |total'|))))\n\
         (assert (forall ((a Int) (i Int) (total Int) (|i'| Int) (|total'| Int)) (=> (and (inv a i total) (> a 0) (<= a 10) (and (not (< i a)) (= |i'| i) (= |total'| total))) (= |total'| (* 10 a)))))\n\
         (check-sat)\n"
    );
}

fn invariant(arena: &mut TermArena, text: &str) -> i64 {
    let i = arena.var("i");
    let total = arena.var("total");
    let a = arena.var("a");
    let ten = arena.int(10);
    let ten_i = arena.mul(ten, i);
    let sum = arena.equal(total, ten_i);
    let below = arena.le(i, a);
    match text {
        "true" => arena.and(vec![sum, below]),
        "not kept" => {
            // `i == 0` holds at entry, and the first turn breaks it.
            let zero = arena.int(0);
            let at_zero = arena.equal(i, zero);
            arena.and(vec![sum, at_zero])
        }
        "too weak" => sum,
        _ => unreachable!(),
    }
}

#[test]
fn a_true_invariant_passes_all_three_queries() {
    let mut arena = TermArena::empty();
    let problem = section_one(&mut arena);
    let inv = invariant(&mut arena, "true");
    let verdict = lg_loop_check(&mut arena, &problem, inv, &lg_proved);
    assert!(verdict.proved(), "{verdict:?}");
}

#[test]
fn an_invariant_a_turn_does_not_keep_fails() {
    let mut arena = TermArena::empty();
    let problem = section_one(&mut arena);
    let inv = invariant(&mut arena, "not kept");
    let verdict = lg_loop_check(&mut arena, &problem, inv, &lg_proved);
    assert!(verdict.holds_at_entry, "{verdict:?}");
    assert!(!verdict.kept_by_a_turn, "{verdict:?}");
    assert!(!verdict.proved());
}

#[test]
fn an_invariant_too_weak_for_the_goal_fails() {
    let mut arena = TermArena::empty();
    let problem = section_one(&mut arena);
    let inv = invariant(&mut arena, "too weak");
    let verdict = lg_loop_check(&mut arena, &problem, inv, &lg_proved);
    assert!(
        verdict.holds_at_entry && verdict.kept_by_a_turn,
        "{verdict:?}"
    );
    assert_eq!(verdict.gives_the_goals, vec![false]);
    assert!(!verdict.proved());
}
