// Two batons for a large state: (a) Option<State> + mem::replace with a hollow
// one, (b) Option<Box<State>> + swap with a heap spare. Max depth on 8 MB.
use std::cell::RefCell;
#[derive(Clone)]
pub struct State { pub findings: Vec<String>, pub seq: i64, pub pad: [u64; 512] }
impl State { fn hollow() -> State { State { findings: Vec::new(), seq: 0, pad: [0; 512] } } }
pub trait Host { fn expr(&self, state: &mut State, depth: i64) -> i64; }
#[inline(never)]
pub fn core_value_of<H: Host>(state: &mut State, host: &H, depth: i64) -> i64 {
    state.seq += 1;
    if depth <= 0 { return 0; }
    host.expr(state, depth - 1) + 1
}
#[cfg(not(boxed))]
mod v {
    use super::*;
    pub struct Baton(pub Option<State>);
    pub struct Checker { pub s: Baton }
    pub struct Core<'c> { c: RefCell<&'c mut Checker> }
    impl Checker {
        #[inline(never)]
        pub fn value_of(&mut self, depth: i64) -> i64 {
            let mut s = self.s.0.take().expect("out");
            let r = { let host = Core { c: RefCell::new(&mut *self) }; core_value_of(&mut s, &host, depth) };
            self.s.0 = Some(s); r
        }
        #[inline(never)]
        pub fn expr(&mut self, depth: i64) -> i64 { self.s.0.as_mut().unwrap().seq += 100; self.value_of(depth) }
    }
    impl Host for Core<'_> {
        fn expr(&self, state: &mut State, depth: i64) -> i64 {
            let mut c = self.c.borrow_mut();
            c.s.0 = Some(std::mem::replace(state, State::hollow()));
            let r = c.expr(depth);
            *state = c.s.0.take().expect("taken");
            r
        }
    }
    pub fn start() -> Checker { Checker { s: Baton(Some(State::hollow())) } }
}
#[cfg(boxed)]
mod v {
    use super::*;
    #[inline(never)]
    #[cold]
    fn spare() -> Box<State> { Box::new(State::hollow()) }
    pub struct Baton(pub Option<Box<State>>);
    pub struct Checker { pub s: Baton }
    pub struct Core<'c> { c: RefCell<&'c mut Checker>, spare: RefCell<Option<Box<State>>> }
    impl Checker {
        #[inline(never)]
        pub fn value_of(&mut self, depth: i64) -> i64 {
            let mut s = self.s.0.take().expect("out");
            let r = { let host = Core { c: RefCell::new(&mut *self), spare: RefCell::new(Some(spare())) }; core_value_of(&mut *s, &host, depth) };
            self.s.0 = Some(s); r
        }
        #[inline(never)]
        pub fn expr(&mut self, depth: i64) -> i64 { self.s.0.as_mut().unwrap().seq += 100; self.value_of(depth) }
    }
    impl Host for Core<'_> {
        fn expr(&self, state: &mut State, depth: i64) -> i64 {
            let mut c = self.c.borrow_mut();
            let mut lent = self.spare.borrow_mut().take().expect("spare");
            std::mem::swap(state, &mut *lent);
            c.s.0 = Some(lent);
            let r = c.expr(depth);
            let mut back = c.s.0.take().expect("taken");
            std::mem::swap(state, &mut *back);
            *self.spare.borrow_mut() = Some(back);
            r
        }
    }
    pub fn start() -> Checker { Checker { s: Baton(Some(Box::new(State::hollow()))) } }
}
fn main() {
    let depth: i64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let t = std::thread::Builder::new().stack_size(8 << 20).spawn(move || {
        let mut c = v::start(); let r = c.expr(depth); (r, c.s.0.as_ref().unwrap().seq)
    }).unwrap();
    println!("{:?}", t.join().unwrap());
}
