    fn somewhere(&self) -> bool {
        self.reads.workshop().somewhere()
    }

    fn decoded(&self, literal: &str) -> Option<String> {
        build_time::decoded(literal)
    }

    fn read(&self, path: &str) -> BtRead {
        match self.reads.read(path) {
            Ok(text) => BtRead {
                text: Some(text),
                denied: None,
            },
            Err(why) => BtRead {
                text: None,
                denied: Some(why),
            },
        }
    }

    fn assets_read_in(&self, value: &Expr, read: &BTreeMap<String, String>) -> Expr {
        let mut value = value.clone();
        if !read.is_empty() {
            let parsed = self.parsed;
            crate::text_tiers::visit_expr_mut(
                &mut value,
                &mut |expr: &mut Expr| {
                    if let Expr::Call { func, args, .. } = expr
                        && matches!(&**func, Expr::Variable(name, _)
                            if parsed.text(*name) == crate::assets::ASSET)
                        && let [Expr::LitStr { text, .. }] = args.as_slice()
                        && let Some(content) =
                            build_time::decoded(text).and_then(|path| read.get(&path))
                    {
                        *expr = Expr::LitStr {
                            text: build_time::written(content),
                            id: crate::ast::NodeId::fresh(),
                        };
                    }
                },
                &mut |_, _| {},
            );
        }
        value
    }

    fn value_fn(&self) -> String {
        crate::comptime_run::VALUE_FN.to_string()
    }

    fn place_of(&self, place: &str, here: i64) -> CheckPlace {
        let (files, _) = self.files();
        let here = here as usize;
        let stopped = Some(place)
            .and_then(|place| place.strip_prefix(crate::comptime_run::STOPPED_AT))
            .and_then(|rest| rest.split_once(':'))
            .and_then(|(unit, byte)| {
                Some((unit.parse::<usize>().ok()?, byte.parse::<u32>().ok()?))
            });
        let at = stopped
            .filter(|(unit, _)| *unit == here)
            .map(|(_, byte)| byte);
        // `@<file>:<byte>`: a place in this file is shown, and one in another
        // is named by the function it is in.
        let function = stopped
            .filter(|(unit, _)| *unit != here)
            .and_then(|(unit, byte)| {
                let file = files.get(unit)?;
                file.program.items.iter().find_map(|item| match &item.node {
                    Item::Fn {
                        name: Some(name), ..
                    } if (item.span.start..item.span.end).contains(&byte) => {
                        Some(file.text(*name).to_string())
                    }
                    _ => None,
                })
            });
        CheckPlace {
            at: at.map_or(-1, i64::from),
            function,
        }
    }

    fn written(&self, value: &Expr) -> String {
        super::written_by(self.parsed, value)
    }

    fn newly_throws(&self, key: &str) -> Option<Vec<String>> {
        self.newly.throws.get(key).cloned()
    }

    /// Code below the core never walks an expression: the hubs are the core's
    /// host's (`core::Core`), until the dispatcher moves (U-c7, U-c11).
