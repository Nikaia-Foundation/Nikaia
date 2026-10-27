fn big(limit: Option<i64>, other: Option<i64>) -> i64 {
    if let Some(limit) = limit && limit > 3 {
        return limit + 1;
    }
    if let Some(n) = other {
        return n * 2;
    }
    0
}

impl Cache {
    fn size(&self) -> usize {
        if let Some(x) = &self.f && x.len() > 2 {
            return x.len();
        }
        0
    }
    fn reset(&mut self) -> usize {
        if let Some(x) = &self.f {
            self.f = None;
            return x.len();
        }
        0
    }
    fn deep(&self, p: &Node) -> String {
        match &p.inner.name {
            Some(name) => format!("{name}"),
            None => String::new(),
        }
    }
}
