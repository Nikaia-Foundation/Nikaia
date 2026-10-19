// crates/nikaia/src/zig.rs
//
// **An experiment: a second back end that writes Zig instead of Rust.**
//
// `nikaia zig <file>` lowers a checked file to one `.zig` file that
// `zig run` compiles. It reads the syntax tree the parser builds, after the
// checker has accepted the file, so what a program means stays the checker's;
// this module only says it in another language. It covers a small part of the
// language - functions over integers, floats, `bool` and `String`, `let`,
// `while`, `for` over a range or a list, `if`, structs with methods, f-strings
// and `println` - and refuses everything else by name.
//
// What carries over without work: Zig's safe builds stop on an overflowing
// `+`, `-` and `*` and on a division by zero, as Nikaia does. What needs work
// is what Zig refuses that Rust only warns about: an unused local, a `var`
// never changed, a name that shadows another. Each binding is therefore
// emitted with markers and settled once its function is written
// ([`Writer::settle`]).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use anyhow::Result;

use crate::ast::{BinaryOp, Block, Expr, FPart, Item, Spanned, Stmt, Type, UnaryOp};
use crate::parser::Parsed;
use crate::refuse;

/// The Zig the file says, as one source file.
pub fn lower(parsed: &Parsed) -> Result<String> {
    let mut module = Module::new(parsed);
    module.collect()?;
    module.emit()
}

/// The Zig version the output is written for: `std.io.getStdOut` moved after it.
pub const ZIG_VERSION: &str = "0.13";

const PRELUDE: &str = "\
// Written by `nikaia zig` (experimental). Zig 0.13.
const nk_std = @import(\"std\");

fn nk_print(comptime format: []const u8, args: anytype) void {
    nk_std.io.getStdOut().writer().print(format, args) catch {};
}
";

struct FnSig {
    params: Vec<String>,
    ret: Option<String>,
    /// A method that changes `self`: its receiver must be a `var`.
    mut_self: bool,
}

struct StructDef {
    fields: Vec<(String, String)>,
}

struct Module<'p> {
    parsed: &'p Parsed,
    fns: HashMap<String, FnSig>,
    /// Methods by struct, then by name.
    methods: HashMap<String, HashMap<String, FnSig>>,
    structs: Vec<(String, StructDef)>,
    globals: HashSet<String>,
}

impl<'p> Module<'p> {
    fn new(parsed: &'p Parsed) -> Self {
        Module {
            parsed,
            fns: HashMap::new(),
            methods: HashMap::new(),
            structs: Vec::new(),
            globals: HashSet::new(),
        }
    }

    fn text(&self, sym: winnow_grammar::Symbol) -> &'p str {
        self.parsed.text(sym)
    }

    fn struct_names(&self) -> HashSet<String> {
        self.structs.iter().map(|(n, _)| n.clone()).collect()
    }

    /// Every signature first, so a call can know what its callee returns.
    fn collect(&mut self) -> Result<()> {
        let parsed = self.parsed;
        for item in &parsed.program.items {
            if let Item::Struct { name, generics, .. } = &item.node {
                if !generics.is_empty() {
                    refuse!("the Zig prototype does not lower generic structs yet");
                }
                let name = self.text(*name).to_string();
                self.globals.insert(name.clone());
                self.structs.push((name, StructDef { fields: Vec::new() }));
            }
        }
        let known = self.struct_names();
        let parsed = self.parsed;
        for item in &parsed.program.items {
            match &item.node {
                Item::Struct { name, fields, .. } => {
                    let mut lowered = Vec::new();
                    for field in fields {
                        lowered.push((
                            self.text(field.name).to_string(),
                            self.zig_type(&field.ty, &known)?,
                        ));
                    }
                    let name = self.text(*name);
                    let def = self.structs.iter_mut().find(|(n, _)| n == name).unwrap();
                    def.1.fields = lowered;
                }
                Item::Fn { name, .. } => {
                    let sig = self.signature(&item.node, &known, None)?;
                    let name = self.text(name.expect("a function at the top has a name"));
                    self.globals.insert(name.to_string());
                    self.fns.insert(name.to_string(), sig);
                }
                Item::Impl {
                    trait_name,
                    target,
                    methods,
                } => {
                    if trait_name.is_some() {
                        refuse!("the Zig prototype does not lower trait impls yet");
                    }
                    let target = self.text(target.name).to_string();
                    if !known.contains(&target) {
                        refuse!("the Zig prototype lowers an `impl` only for a struct of the file");
                    }
                    for method in methods {
                        let Item::Fn { name, .. } = &method.node else {
                            continue;
                        };
                        let sig = self.signature(&method.node, &known, Some(&target))?;
                        let name = self.text(name.expect("a method has a name")).to_string();
                        self.methods
                            .entry(target.clone())
                            .or_default()
                            .insert(name, sig);
                    }
                }
                Item::Import { .. } => {
                    refuse!("the Zig prototype does not lower `use` yet: one file, no packages")
                }
                Item::Test { .. } | Item::Bench { .. } => {}
                _ => refuse!(
                    "the Zig prototype lowers functions, structs and their impls; this file \
                     declares something else (an enum, a trait, an extern block, a grammar or \
                     a build-time value)"
                ),
            }
        }
        Ok(())
    }

    fn signature(
        &self,
        item: &Item,
        known: &HashSet<String>,
        owner: Option<&str>,
    ) -> Result<FnSig> {
        let Item::Fn {
            generics,
            receiver,
            args,
            ret_type,
            can_throw,
            ..
        } = item
        else {
            unreachable!("a signature is a function's")
        };
        if !generics.is_empty() {
            refuse!("the Zig prototype does not lower generic functions yet");
        }
        if *can_throw {
            refuse!("the Zig prototype does not lower `throws` yet");
        }
        let mut params = Vec::new();
        if let (Some(receiver), Some(owner)) = (receiver, owner) {
            params.push(if receiver.is_mut {
                format!("*{owner}")
            } else {
                owner.to_string()
            });
        }
        for arg in args {
            if arg.mutable {
                refuse!("the Zig prototype does not lower `mut` parameters yet");
            }
            params.push(self.zig_type(&arg.ty, known)?);
        }
        let ret = match ret_type {
            Some(ty) => Some(self.zig_type(ty, known)?),
            None => None,
        };
        Ok(FnSig {
            params,
            ret,
            mut_self: receiver.as_ref().is_some_and(|r| r.is_mut),
        })
    }

    fn zig_type(&self, ty: &Type, known: &HashSet<String>) -> Result<String> {
        let name = self.text(ty.name);
        if ty.is_tuple || ty.is_nullable || ty.code.is_some() || ty.count.is_some() {
            refuse!("the Zig prototype does not lower the type `{name}` in this shape yet");
        }
        Ok(match name {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
            | "f32" | "f64" | "bool" | "isize" | "usize" => name.to_string(),
            "String" | "str" => "[]const u8".to_string(),
            "Vec" if ty.generics.len() == 1 => {
                format!("[]const {}", self.zig_type(&ty.generics[0], known)?)
            }
            "Self" => refuse!("the Zig prototype does not lower `Self` yet: name the struct"),
            _ if known.contains(name) => name.to_string(),
            _ => refuse!("the Zig prototype does not lower the type `{name}` yet"),
        })
    }

    fn emit(&self) -> Result<String> {
        let mut out = String::from(PRELUDE);
        for (name, def) in &self.structs {
            let _ = writeln!(out, "\nconst {} = struct {{", ident(name));
            for (field, ty) in &def.fields {
                let _ = writeln!(out, "    {}: {ty},", ident(field));
            }
            for item in &self.parsed.program.items {
                let Item::Impl {
                    target, methods, ..
                } = &item.node
                else {
                    continue;
                };
                if self.text(target.name) != name {
                    continue;
                }
                for method in methods {
                    out.push('\n');
                    for line in self.function(&method.node, Some(name))?.lines() {
                        if line.is_empty() {
                            out.push('\n');
                        } else {
                            let _ = writeln!(out, "    {line}");
                        }
                    }
                }
            }
            out.push_str("};\n");
        }
        for item in &self.parsed.program.items {
            if let Item::Fn { .. } = &item.node {
                out.push('\n');
                out.push_str(&self.function(&item.node, None)?);
            }
        }
        Ok(out)
    }

    fn function(&self, item: &Item, owner: Option<&str>) -> Result<String> {
        let Item::Fn {
            name,
            receiver,
            args,
            body,
            ..
        } = item
        else {
            unreachable!("a function is a function")
        };
        let name = self.text(name.expect("a function has a name"));
        let sig = match owner {
            Some(owner) => &self.methods[owner][name],
            None => &self.fns[name],
        };
        let mut f = Writer::new(self, sig.ret.clone());
        let mut params = Vec::new();
        let mut types = sig.params.iter();
        if receiver.is_some() {
            let ty = types.next().expect("a receiver has a type").clone();
            let zig = f.declare("self", ty.trim_start_matches('*').to_string(), Kind::Param);
            params.push(format!("{zig}: {ty}"));
        }
        for (arg, ty) in args.iter().zip(types) {
            let zig = f.declare(self.text(arg.name), ty.clone(), Kind::Param);
            params.push(format!("{zig}: {ty}"));
        }
        let is_main = owner.is_none() && name == "main";
        if is_main && (!params.is_empty() || sig.ret.is_some()) {
            refuse!("the Zig prototype lowers only `fn main()` without arguments or result");
        }
        let ret = sig.ret.clone().unwrap_or_else(|| "void".to_string());
        let public = if is_main || owner.is_some() {
            "pub "
        } else {
            ""
        };
        let mut text = format!(
            "{public}fn {}({}) {ret} {{\n",
            ident(name),
            params.join(", ")
        );
        // The parameters' markers stand before the body, which may use them.
        for index in 0..f.bindings.len() {
            text.push_str(&format!("    \u{1}U{index}\u{1}\n"));
        }
        f.block_stmts(body, 1, &mut text)?;
        text.push_str("}\n");
        Ok(f.settle(text))
    }
}

/// What a name was bound by, which decides what Zig needs said about it.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Param,
    Let { mutable: bool },
    Capture,
}

struct Binding {
    zig: String,
    ty: String,
    kind: Kind,
    used: bool,
    changed: bool,
}

/// One function being written.
struct Writer<'m, 'p> {
    module: &'m Module<'p>,
    ret: Option<String>,
    /// The type the next `let` takes when its value is only literals.
    hint: Option<String>,
    bindings: Vec<Binding>,
    /// What a source name means here, innermost scope last.
    scopes: Vec<HashMap<String, usize>>,
    /// Every Zig name taken in this function: Zig forbids shadowing.
    taken: HashSet<String>,
}

impl<'m, 'p> Writer<'m, 'p> {
    fn new(module: &'m Module<'p>, ret: Option<String>) -> Self {
        Writer {
            module,
            ret,
            hint: None,
            bindings: Vec::new(),
            scopes: vec![HashMap::new()],
            taken: HashSet::new(),
        }
    }

    fn text(&self, sym: winnow_grammar::Symbol) -> &'p str {
        self.module.text(sym)
    }

    fn declare(&mut self, name: &str, ty: String, kind: Kind) -> String {
        let mut zig = ident(name);
        let mut n = 2;
        while self.taken.contains(&zig) || (name != "self" && self.module.globals.contains(&zig)) {
            zig = ident(&format!("{name}_{n}"));
            n += 1;
        }
        self.taken.insert(zig.clone());
        let index = self.bindings.len();
        self.bindings.push(Binding {
            zig: zig.clone(),
            ty,
            kind,
            used: false,
            changed: false,
        });
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), index);
        format!("\u{1}N{index}\u{1}")
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    /// Replaces the markers now that every use is known: an unused binding
    /// is discarded with `_ = x;` (a capture becomes `_`), and a `let mut`
    /// that nothing changed is a `const`.
    fn settle(&self, mut text: String) -> String {
        for (index, b) in self.bindings.iter().enumerate() {
            let name = if b.kind == Kind::Capture && !b.used {
                "_".to_string()
            } else {
                b.zig.clone()
            };
            text = text.replace(&format!("\u{1}N{index}\u{1}"), &name);
            let keyword = match b.kind {
                Kind::Let { mutable: true } if b.changed => "var",
                _ => "const",
            };
            text = text.replace(&format!("\u{1}K{index}\u{1}"), keyword);
            let discard = match b.kind {
                Kind::Param | Kind::Let { .. } if !b.used => format!("_ = {};", b.zig),
                _ => String::new(),
            };
            text = text.replace(&format!("\u{1}U{index}\u{1}"), &discard);
        }
        // A marker stands on a line of its own; an empty one leaves only the
        // indentation behind.
        text.lines()
            .filter(|l| !l.trim().is_empty() || l.is_empty())
            .fold(String::new(), |mut s, l| {
                s.push_str(l);
                s.push('\n');
                s
            })
    }

    fn block_stmts(&mut self, block: &Block, depth: usize, out: &mut String) -> Result<()> {
        self.scopes.push(HashMap::new());
        for (at, stmt) in block.stmts.iter().enumerate() {
            if let Stmt::Let {
                names,
                ty: None,
                value,
                ..
            } = &stmt.node
                && untyped(value)
            {
                self.hint = self.hint_in(names[0], &block.stmts[at + 1..]);
            }
            self.stmt(&stmt.node, depth, out)?;
            self.hint = None;
        }
        self.scopes.pop();
        Ok(())
    }

    /// What the statements after a `let` say the name's type is, where its
    /// value is only literals: Nikaia takes it from the uses, the checker's
    /// way; this looks at a `return`, an assignment and a call's argument.
    fn hint_in(&self, name: winnow_grammar::Symbol, stmts: &[Spanned<Stmt>]) -> Option<String> {
        let is_it = |e: &Expr| matches!(e, Expr::Variable(n, _) if *n == name);
        for stmt in stmts {
            let found = match &stmt.node {
                Stmt::Return(Some(e)) if is_it(e) => self.ret.clone(),
                Stmt::Assign { target, value, .. } if is_it(target) && !untyped(value) => {
                    self.type_of(value)
                }
                Stmt::Assign { target, value, .. } if is_it(value) => self.type_of(target),
                Stmt::Let { names, value, .. } if names[0] == name => {
                    // Shadowed from here on; its own value may still read it.
                    return self.hint_in_expr(&is_it, value);
                }
                Stmt::Let { value, .. } | Stmt::Expr(value) => self.hint_in_expr(&is_it, value),
                Stmt::While { body, .. } => self.hint_in(name, &body.stmts),
                Stmt::For { body, .. } => self.hint_in(name, &body.stmts),
                _ => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }

    fn hint_in_expr(&self, is_it: &dyn Fn(&Expr) -> bool, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Call { func, args, .. } => {
                let Expr::Variable(callee, _) = &**func else {
                    return None;
                };
                let sig = self.module.fns.get(self.text(*callee))?;
                args.iter()
                    .position(is_it)
                    .and_then(|i| sig.params.get(i).cloned())
            }
            Expr::If {
                then_branch,
                else_branch,
                ..
            } => then_branch
                .stmts
                .iter()
                .chain(else_branch.iter().flat_map(|b| b.stmts.iter()))
                .find_map(|s| match &s.node {
                    Stmt::Return(Some(e)) if is_it(e) => self.ret.clone(),
                    Stmt::Expr(e) | Stmt::Let { value: e, .. } => self.hint_in_expr(is_it, e),
                    _ => None,
                }),
            _ => None,
        }
    }

    fn line(&self, depth: usize, out: &mut String, text: &str) {
        for _ in 0..depth {
            out.push_str("    ");
        }
        out.push_str(text);
        out.push('\n');
    }

    fn stmt(&mut self, stmt: &Stmt, depth: usize, out: &mut String) -> Result<()> {
        match stmt {
            Stmt::Let {
                names,
                mutable,
                ty,
                value,
                ..
            } => {
                if names.len() != 1 {
                    refuse!("the Zig prototype does not lower destructuring `let` yet");
                }
                let known = self.module.struct_names();
                let declared = match ty {
                    Some(ty) => Some(self.module.zig_type(ty, &known)?),
                    None => None,
                };
                let value_text = self.expr(value)?;
                let ty = match declared
                    .or_else(|| self.hint.take())
                    .or_else(|| self.type_of(value))
                {
                    Some(ty) => ty,
                    None => refuse!(
                        "the Zig prototype cannot tell the type of `{}`: write it down",
                        self.text(names[0])
                    ),
                };
                // The value is read before the name exists: `let x = x + 1`.
                let index = self.bindings.len();
                let name = self.declare(
                    self.text(names[0]),
                    ty.clone(),
                    Kind::Let { mutable: *mutable },
                );
                self.line(
                    depth,
                    out,
                    &format!("\u{1}K{index}\u{1} {name}: {ty} = {value_text};"),
                );
                self.line(depth, out, &format!("\u{1}U{index}\u{1}"));
            }
            Stmt::Assign { target, op, value } => {
                self.mark_changed(target)?;
                let target_text = self.expr(target)?;
                let value_text = self.expr(value)?;
                let text = match op {
                    None => format!("{target_text} = {value_text};"),
                    Some(BinaryOp::Div | BinaryOp::Rem) => {
                        let op = op.unwrap();
                        let float = self.type_of(target).is_some_and(|t| t.starts_with('f'));
                        format!(
                            "{target_text} = {};",
                            arithmetic(op, &target_text, &value_text, float)
                        )
                    }
                    Some(op) => format!("{target_text} {}= {value_text};", operator(*op)),
                };
                self.line(depth, out, &text);
            }
            Stmt::For {
                bindings,
                iter,
                body,
                ..
            } => {
                if bindings.len() != 1 {
                    refuse!("the Zig prototype does not lower destructuring `for` yet");
                }
                let name = self.text(bindings[0]);
                self.line(depth, out, "{");
                self.scopes.push(HashMap::new());
                if let Expr::Range {
                    start,
                    end,
                    inclusive,
                    ..
                } = iter
                {
                    let ty = self
                        .type_of(start)
                        .or_else(|| self.type_of(end))
                        .unwrap_or_else(|| "i64".to_string());
                    let start = self.expr(start)?;
                    let end_value = self.expr(end)?;
                    let end_name = self.declare(&format!("{name}_end"), ty.clone(), Kind::Capture);
                    let end_index = self.bindings.len() - 1;
                    self.bindings[end_index].used = true;
                    self.line(
                        depth + 1,
                        out,
                        &format!("const {end_name}: {ty} = {end_value};"),
                    );
                    let index = self.bindings.len();
                    let var = self.declare(name, ty.clone(), Kind::Capture);
                    // The counter is read by the loop's own test.
                    self.bindings[index].used = true;
                    let test = if *inclusive { "<=" } else { "<" };
                    self.line(depth + 1, out, &format!("var {var}: {ty} = {start};"));
                    self.line(
                        depth + 1,
                        out,
                        &format!("while ({var} {test} {end_name}) : ({var} += 1) {{"),
                    );
                } else {
                    let Some(list) = self.type_of(iter) else {
                        refuse!("the Zig prototype cannot tell what this `for` walks");
                    };
                    let Some(element) = list.strip_prefix("[]const ") else {
                        refuse!("the Zig prototype walks only ranges and lists with `for`");
                    };
                    let iter_text = self.expr(iter)?;
                    let var = self.declare(name, element.to_string(), Kind::Capture);
                    self.line(depth + 1, out, &format!("for ({iter_text}) |{var}| {{"));
                }
                self.block_stmts(body, depth + 2, out)?;
                self.scopes.pop();
                self.line(depth + 1, out, "}");
                self.line(depth, out, "}");
            }
            Stmt::While { cond, body } => {
                let cond = self.expr(cond)?;
                self.line(depth, out, &format!("while ({cond}) {{"));
                self.block_stmts(body, depth + 1, out)?;
                self.line(depth, out, "}");
            }
            Stmt::Return(value) => {
                let text = match value {
                    Some(value) => format!("return {};", self.expr(value)?),
                    None => "return;".to_string(),
                };
                self.line(depth, out, &text);
            }
            Stmt::Break => self.line(depth, out, "break;"),
            Stmt::Continue => self.line(depth, out, "continue;"),
            Stmt::Expr(expr) => self.expr_stmt(expr, depth, out)?,
            Stmt::Comptime { .. } => {
                refuse!("the Zig prototype does not lower build-time values yet")
            }
        }
        Ok(())
    }

    fn expr_stmt(&mut self, expr: &Expr, depth: usize, out: &mut String) -> Result<()> {
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond = self.expr(cond)?;
                self.line(depth, out, &format!("if ({cond}) {{"));
                self.block_stmts(then_branch, depth + 1, out)?;
                if let Some(else_branch) = else_branch {
                    self.line(depth, out, "} else {");
                    self.block_stmts(else_branch, depth + 1, out)?;
                }
                self.line(depth, out, "}");
            }
            Expr::Block(block, _) => {
                self.line(depth, out, "{");
                self.block_stmts(block, depth + 1, out)?;
                self.line(depth, out, "}");
            }
            Expr::Return(value, _) => {
                return self.stmt(&Stmt::Return((**value).clone()), depth, out);
            }
            Expr::Break(_) => self.line(depth, out, "break;"),
            Expr::Continue(_) => self.line(depth, out, "continue;"),
            _ => {
                let text = self.expr(expr)?;
                // Zig refuses a value nobody reads.
                let discard = match self.type_of(expr) {
                    Some(ty) if ty != "void" => "_ = ",
                    _ => "",
                };
                self.line(depth, out, &format!("{discard}{text};"));
            }
        }
        Ok(())
    }

    fn mark_changed(&mut self, target: &Expr) -> Result<()> {
        match target {
            Expr::Variable(name, _) => {
                let name = self.text(*name);
                let Some(index) = self.lookup(name) else {
                    refuse!("the Zig prototype does not know `{name}`");
                };
                self.bindings[index].changed = true;
            }
            Expr::Field { base, .. } => self.mark_changed(base)?,
            _ => refuse!("the Zig prototype assigns only to a local or a field of one"),
        }
        Ok(())
    }

    /// The Zig type an expression has, where this prototype can tell.
    fn type_of(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::LitInt { .. } => Some("i64".to_string()),
            Expr::LitFloat(..) => Some("f64".to_string()),
            Expr::LitBool(..) => Some("bool".to_string()),
            Expr::LitStr { .. } | Expr::LitInterpolated { .. } => Some("[]const u8".to_string()),
            Expr::Variable(name, _) => {
                let index = self.lookup(self.text(*name))?;
                Some(self.bindings[index].ty.clone())
            }
            Expr::ListLit { items, .. } => {
                let element = items
                    .first()
                    .and_then(|i| self.type_of(i))
                    .unwrap_or_else(|| "i64".to_string());
                Some(format!("[]const {element}"))
            }
            Expr::Binary { op, lhs, rhs, .. } => match op {
                BinaryOp::Eq
                | BinaryOp::Ne
                | BinaryOp::Lt
                | BinaryOp::Le
                | BinaryOp::Gt
                | BinaryOp::Ge
                | BinaryOp::And
                | BinaryOp::Or => Some("bool".to_string()),
                // A literal takes the other side's type.
                _ => match (&**lhs, &**rhs) {
                    (Expr::LitInt { .. } | Expr::LitFloat(..), other) => self.type_of(other),
                    (other, _) => self.type_of(other),
                },
            },
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Not => Some("bool".to_string()),
                _ => self.type_of(expr),
            },
            Expr::Call { func, .. } => match &**func {
                Expr::Variable(name, _) => {
                    let name = self.text(*name);
                    match name {
                        "println" | "print" => Some("void".to_string()),
                        _ => Some(
                            self.module
                                .fns
                                .get(name)?
                                .ret
                                .clone()
                                .unwrap_or_else(|| "void".into()),
                        ),
                    }
                }
                Expr::Path(path, _) if path.len() == 2 => {
                    let sig = self
                        .module
                        .methods
                        .get(self.text(path[0]))?
                        .get(self.text(path[1]))?;
                    Some(sig.ret.clone().unwrap_or_else(|| "void".into()))
                }
                _ => None,
            },
            Expr::MethodCall {
                receiver, method, ..
            } => {
                let method = self.text(*method);
                let owner = self.type_of(receiver)?;
                if method == "len" && (owner.starts_with("[]")) {
                    return Some("i64".to_string());
                }
                let sig = self
                    .module
                    .methods
                    .get(owner.trim_start_matches('*'))?
                    .get(method)?;
                Some(sig.ret.clone().unwrap_or_else(|| "void".into()))
            }
            Expr::Field { base, name, .. } => {
                let owner = self.type_of(base)?;
                let (_, def) = self
                    .module
                    .structs
                    .iter()
                    .find(|(n, _)| *n == owner.trim_start_matches('*'))?;
                let name = self.text(*name);
                def.fields
                    .iter()
                    .find(|(f, _)| f == name)
                    .map(|(_, t)| t.clone())
            }
            Expr::StructLit { name, .. } => Some(self.text(*name).to_string()),
            Expr::Index { base, .. } => {
                let list = self.type_of(base)?;
                list.strip_prefix("[]const ").map(str::to_string)
            }
            Expr::Cast { ty, .. } => self.module.zig_type(ty, &self.module.struct_names()).ok(),
            Expr::If { then_branch, .. } => match then_branch.stmts.as_slice() {
                [only] => match &only.node {
                    Stmt::Expr(e) => self.type_of(e),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn expr(&mut self, expr: &Expr) -> Result<String> {
        Ok(match expr {
            Expr::LitInt {
                value, negative, ..
            } => {
                if *negative {
                    format!("-{value}")
                } else {
                    value.to_string()
                }
            }
            Expr::LitFloat(text, _) => text.clone(),
            Expr::LitBool(b, _) => b.to_string(),
            Expr::LitStr { text, .. } => string_literal(text),
            Expr::LitInterpolated { parts, .. } => {
                let (format, args) = self.format(parts)?;
                // An f-string as a value is formatted into memory nobody frees:
                // this prototype has no allocator story yet.
                format!(
                    "(nk_std.fmt.allocPrint(nk_std.heap.page_allocator, {format}, .{{{args}}}) catch @panic(\"out of memory\"))"
                )
            }
            Expr::Variable(name, _) => {
                let name = self.text(*name);
                let Some(index) = self.lookup(name) else {
                    refuse!("the Zig prototype does not know `{name}` here");
                };
                self.bindings[index].used = true;
                format!("\u{1}N{index}\u{1}")
            }
            Expr::ListLit { items, .. } => {
                let element = self
                    .type_of(expr)
                    .and_then(|t| t.strip_prefix("[]const ").map(str::to_string))
                    .unwrap_or_else(|| "i64".to_string());
                let items = items
                    .iter()
                    .map(|i| self.expr(i))
                    .collect::<Result<Vec<_>>>()?;
                format!("&[_]{element}{{ {} }}", items.join(", "))
            }
            Expr::Binary { op, lhs, rhs, .. } => {
                let float = self.type_of(expr).is_some_and(|t| t.starts_with('f'));
                let text = self.type_of(lhs).is_some_and(|t| t == "[]const u8");
                let l = self.operand(lhs)?;
                let r = self.operand(rhs)?;
                match op {
                    BinaryOp::Eq | BinaryOp::Ne if text => {
                        let not = if *op == BinaryOp::Ne { "!" } else { "" };
                        format!("{not}nk_std.mem.eql(u8, {l}, {r})")
                    }
                    BinaryOp::Div | BinaryOp::Rem => arithmetic(*op, &l, &r, float),
                    BinaryOp::Shl | BinaryOp::Shr => {
                        format!("{l} {} @intCast({r})", operator(*op))
                    }
                    _ => format!("{l} {} {r}", operator(*op)),
                }
            }
            Expr::Unary { op, expr, .. } => match op {
                UnaryOp::Neg => format!("-{}", self.operand(expr)?),
                UnaryOp::Not => format!("!{}", self.operand(expr)?),
                UnaryOp::Ref => self.expr(expr)?,
            },
            Expr::Call {
                func, args, config, ..
            } => {
                if !config.is_empty() {
                    refuse!("the Zig prototype does not lower a call's options yet");
                }
                match &**func {
                    Expr::Variable(name, _) => {
                        let name = self.text(*name);
                        match name {
                            "println" | "print" => return self.print(name == "println", args),
                            _ if self.module.fns.contains_key(name) => {
                                let args = self.args(args)?;
                                format!("{}({args})", ident(name))
                            }
                            _ => refuse!("the Zig prototype does not lower a call of `{name}` yet"),
                        }
                    }
                    Expr::Path(path, _) if path.len() == 2 => {
                        let owner = self.text(path[0]);
                        let method = self.text(path[1]);
                        if !self
                            .module
                            .methods
                            .get(owner)
                            .is_some_and(|m| m.contains_key(method))
                        {
                            refuse!(
                                "the Zig prototype does not lower a call of `{owner}::{method}` yet"
                            );
                        }
                        let args = self.args(args)?;
                        format!("{}.{}({args})", ident(owner), ident(method))
                    }
                    _ => refuse!("the Zig prototype calls only named functions"),
                }
            }
            Expr::MethodCall {
                receiver,
                method,
                args,
                config,
                ..
            } => {
                if !config.is_empty() {
                    refuse!("the Zig prototype does not lower a call's options yet");
                }
                let method = self.text(*method);
                let Some(owner) = self.type_of(receiver) else {
                    refuse!("the Zig prototype cannot tell what `.{method}()` is called on");
                };
                if method == "len" && owner.starts_with("[]") && args.is_empty() {
                    return Ok(format!(
                        "@as(i64, @intCast({}.len))",
                        self.operand(receiver)?
                    ));
                }
                let Some(sig) = self
                    .module
                    .methods
                    .get(owner.trim_start_matches('*'))
                    .and_then(|m| m.get(method))
                else {
                    refuse!("the Zig prototype does not lower `.{method}()` on `{owner}` yet");
                };
                if sig.mut_self {
                    self.mark_changed(receiver)?;
                }
                let receiver = self.operand(receiver)?;
                let args = self.args(args)?;
                format!("{receiver}.{}({args})", ident(method))
            }
            Expr::Field { base, name, .. } => {
                format!("{}.{}", self.operand(base)?, ident(self.text(*name)))
            }
            Expr::StructLit { name, fields, .. } => {
                let mut inits = Vec::new();
                for field in fields {
                    let field_name = self.text(field.name);
                    let value = match &field.value {
                        Some(value) => self.expr(value)?,
                        None => {
                            self.expr(&Expr::Variable(field.name, crate::ast::NodeId::fresh()))?
                        }
                    };
                    inits.push(format!(".{} = {value}", ident(field_name)));
                }
                format!("{}{{ {} }}", ident(self.text(*name)), inits.join(", "))
            }
            Expr::Index { base, index, .. } => {
                format!("{}[@intCast({})]", self.operand(base)?, self.expr(index)?)
            }
            Expr::Cast {
                expr: inner, ty, ..
            } => {
                let ty = self.module.zig_type(ty, &self.module.struct_names())?;
                let from = self.type_of(inner).unwrap_or_default();
                let value = self.expr(inner)?;
                match (from.starts_with('f'), ty.starts_with('f')) {
                    (false, true) => format!("@as({ty}, @floatFromInt({value}))"),
                    (true, false) => format!("@as({ty}, @intFromFloat({value}))"),
                    (true, true) => format!("@as({ty}, @floatCast({value}))"),
                    (false, false) => format!("@as({ty}, @intCast({value}))"),
                }
            }
            Expr::If {
                cond,
                then_branch,
                else_branch: Some(else_branch),
                ..
            } => {
                let cond = self.expr(cond)?;
                let then = self.value_of_block(then_branch)?;
                let other = self.value_of_block(else_branch)?;
                format!("(if ({cond}) {then} else {other})")
            }
            _ => refuse!("the Zig prototype does not lower this expression yet"),
        })
    }

    /// A branch of an `if` used as a value: one expression, or an `if` again.
    fn value_of_block(&mut self, block: &Block) -> Result<String> {
        match block.stmts.as_slice() {
            [only] => match &only.node {
                Stmt::Expr(e) => self.expr(e),
                _ => refuse!(
                    "the Zig prototype lowers an `if` value only with one expression a branch"
                ),
            },
            _ => {
                refuse!("the Zig prototype lowers an `if` value only with one expression a branch")
            }
        }
    }

    /// An expression in an operator's place, in parentheses where it has an operator of its own.
    fn operand(&mut self, expr: &Expr) -> Result<String> {
        let text = self.expr(expr)?;
        Ok(match expr {
            Expr::Binary { .. } | Expr::Unary { .. } => format!("({text})"),
            _ => text,
        })
    }

    fn args(&mut self, args: &[Expr]) -> Result<String> {
        Ok(args
            .iter()
            .map(|a| self.expr(a))
            .collect::<Result<Vec<_>>>()?
            .join(", "))
    }

    fn print(&mut self, newline: bool, args: &[Expr]) -> Result<String> {
        let [arg] = args else {
            refuse!("the Zig prototype lowers `println` with one argument");
        };
        let (mut format, args) = match arg {
            Expr::LitInterpolated { parts, .. } => self.format(parts)?,
            other => {
                let spec = spec_for(self.type_of(other).as_deref());
                (format!("\"{spec}\""), self.expr(other)?)
            }
        };
        if newline {
            format.insert_str(format.len() - 1, "\\n");
        }
        Ok(format!("nk_print({format}, .{{{args}}})"))
    }

    /// An f-string as a Zig format string and the tuple's members.
    fn format(&mut self, parts: &[FPart]) -> Result<(String, String)> {
        let mut format = String::new();
        let mut args = Vec::new();
        for part in parts {
            match part {
                // The parser keeps a text part as a format string says it,
                // `{{` for a brace, and Zig reads braces as Rust does.
                FPart::Text(text) => format.push_str(&escaped(text)),
                FPart::Hole { expr, spec, .. } => {
                    if spec.is_some() {
                        refuse!(
                            "the Zig prototype does not lower a format spec in an f-string yet"
                        );
                    }
                    let ty = self.type_of(expr);
                    format.push_str(spec_for(ty.as_deref()));
                    args.push(self.expr(expr)?);
                }
            }
        }
        Ok((format!("\"{format}\""), args.join(", ")))
    }
}

/// A value made of integer literals only, whose type its uses decide.
fn untyped(expr: &Expr) -> bool {
    match expr {
        Expr::LitInt { .. } => true,
        Expr::Unary { expr, .. } => untyped(expr),
        Expr::Binary { lhs, rhs, .. } => untyped(lhs) && untyped(rhs),
        _ => false,
    }
}

fn spec_for(ty: Option<&str>) -> &'static str {
    match ty {
        Some("[]const u8") => "{s}",
        Some("bool") => "{}",
        Some(t) if t.starts_with('i') || t.starts_with('u') || t.starts_with('f') => "{d}",
        _ => "{any}",
    }
}

fn operator(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

/// `/` and `%` round toward zero, as Rust's do; Zig makes a signed integer say so.
fn arithmetic(op: BinaryOp, l: &str, r: &str, float: bool) -> String {
    match (op, float) {
        (BinaryOp::Div, true) => format!("{l} / {r}"),
        (BinaryOp::Div, false) => format!("@divTrunc({l}, {r})"),
        _ => format!("@rem({l}, {r})"),
    }
}

/// A Zig string literal.
fn string_literal(text: &str) -> String {
    format!("\"{}\"", escaped(text))
}

/// A string literal's inside.
fn escaped(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

const ZIG_KEYWORDS: &[&str] = &[
    "addrspace",
    "align",
    "allowzero",
    "and",
    "anyframe",
    "anytype",
    "asm",
    "async",
    "await",
    "break",
    "callconv",
    "catch",
    "comptime",
    "const",
    "continue",
    "defer",
    "else",
    "enum",
    "errdefer",
    "error",
    "export",
    "extern",
    "fn",
    "for",
    "if",
    "inline",
    "linksection",
    "noalias",
    "noinline",
    "nosuspend",
    "opaque",
    "or",
    "orelse",
    "packed",
    "pub",
    "resume",
    "return",
    "struct",
    "suspend",
    "switch",
    "test",
    "threadlocal",
    "try",
    "union",
    "unreachable",
    "usingnamespace",
    "var",
    "volatile",
    "while",
    "type",
    "void",
    "bool",
    "anyerror",
    "noreturn",
    "true",
    "false",
    "null",
    "undefined",
    "isize",
    "usize",
    "comptime_int",
    "comptime_float",
    "anyopaque",
];

/// A name Zig reads as written, or quoted when Zig keeps it for itself.
fn ident(name: &str) -> String {
    let primitive = name.len() > 1
        && (name.starts_with('i') || name.starts_with('u') || name.starts_with('f'))
        && name[1..].chars().all(|c| c.is_ascii_digit());
    if ZIG_KEYWORDS.contains(&name) || primitive {
        format!("@\"{name}\"")
    } else {
        name.to_string()
    }
}
