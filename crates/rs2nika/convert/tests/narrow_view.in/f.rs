pub struct Seeds {
    tables: SeedTables,
    shapes: BTreeMap<String, Shape>,
}

fn names_of(parsed: &Parsed) -> Vec<String> {
    let mut out = Vec::new();
    for item in &parsed.program.items {
        out.push(parsed.text(*item.name).to_string());
    }
    out
}

impl Seeds {
    /// How often a type names a map.
    fn count(&self, parsed: &Parsed, ty: &Type) -> usize {
        self.tables.count(&parsed.interner, ty)
    }

    fn read(&mut self, units: &[&Parsed]) {
        for parsed in units {
            let held = vec!["self"];
            let n: usize = self.count(parsed, &parsed.program.items[0].ty);
            if n > 0 {
                self.tables.found = true;
            }
        }
    }
}
