fn probe(o: Option<String>, n: Option<i64>) -> bool {
    if n.is_some_and(|k| *k > 3) {
        return true;
    }
    if o.as_ref().is_some_and(|s| self.known(s)) {
        return true;
    }
    o.is_some_and(is_word)
}
