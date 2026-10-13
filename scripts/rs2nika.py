#!/usr/bin/env python3
"""rs2nika: a first-draft rewrite of compiler Rust into tools/*.nika.

Not a transpiler: it applies the mechanical rules (R1-R40 below) line by line,
with a little brace tracking for `if let` and `matches!`, and marks what it
cannot rewrite with `// TODO(rs2nika): ...` on the line above. The builder
finishes the draft by hand; `nikaia lower-std` is the judge.

usage:
  rs2nika.py [--profile check|emit|none] [--prefix NAME_] [--stats] FILE.rs|-
      FILE holds one or more Rust items (fns, structs, enums, impl blocks)
      cut out of the compiler. Output: the draft on stdout.
  rs2nika.py ... --file     a whole tools file (use line, free fns, impl <State>)
  rs2nika.py ... --repo DIR name tables from DIR (default: the git checkout of the cwd)
  rs2nika.py --lint F.nika  flag known lowering defects and syntax gaps in hand-written Nikaia
  rs2nika.py --rules        print the rule list
Missing callees (neither in tools/*.nika nor in the input) are listed on stderr.

Profiles map the compiler's context onto the Nikaia state structs:
  check: `self.parsed.text(x)` -> `world.words.resolve(x)`, `self.s.f` -> `self.f`,
         methods get `world: ref CheckWorld` after `self`, impl is `impl CheckState`.
  emit:  the same with `EmitWorld` / `impl EmitState`.
"""
import re
import sys
from collections import Counter

RULES = {
    'R1': 'statement `;` dropped',
    'R2': '`&self` -> `ref self`, `&mut self` -> `ref mut self`',
    'R3': 'param `x: &mut T` -> `mut x: T`; `x: &T` -> `x: ref T`',
    'R4': '`&str`/`&String` -> `ref String`, `&[T]` -> `ref Vec[T]`, `Cow<str>` -> `String`',
    'R5': 'lifetimes dropped (`<\'a>`, `&\'a`, `\'_`)',
    'R6': '`Vec<T>` -> `Vec[T]` (any generic `<..>` in a type -> `[..]`)',
    'R7': '`Option<T>` -> `T?`',
    'R8': '`HashMap`/`HashSet`/`BTreeMap`/`BTreeSet` -> `collections::BTreeMap`/`BTreeSet` (TrustedMap otherwise)',
    'R9': '`Box<T>` -> `T`, `Box::new(x)` -> `x`',
    'R10': '`usize`/`u32`/`u16`/`isize`/`i32` -> `i64` in types; `as usize` casts dropped',
    'R11': 'signature: ` sync` added; `-> Result<T>` -> `-> T sync throws`',
    'R12': '`None` -> `null`, `Some(x)` as a value -> `x`',
    'R13': '`_ =>` -> `else =>`',
    'R14': '`matches!(e, P)` / `matches!(e, P if g)` -> `match e { P => true, else => false }`',
    'R15': '`format!("..{}..", a)` -> `f"..{a}.."`; `&format!` -> `f".."`',
    'R16': '`.to_owned()`/`.into_owned()`/`String::from(x)` -> `.clone()`; `"lit".to_string()` -> `"lit"`; `.to_string()` after a name lookup -> `.clone()`',
    'R17': '`String::new()` -> `""`, `Vec::new()`/`vec![]` -> `[]`, `vec![a, b]` -> `[a, b]`, `BTreeMap::new()` -> `collections::BTreeMap()`',
    'R18': '`*x` deref dropped',
    'R19': '`&x`/`&mut x` at a use dropped (Nikaia lends implicitly)',
    'R20': '`.unwrap_or(d)` / `.unwrap_or_else(|| d)` -> `?? d`; `.cloned()` -> `?.clone()`; `.copied()` dropped',
    'R21': '`.is_some()` -> `!= null`, `.is_none()` -> `== null`',
    'R22': '`let Some(x) = e else { return y };` -> `let x = e ?? return y` (also continue/break)',
    'R23': '`if let Some(x) = e {` -> `if e != null {` + `let x = e ?? panic(..)`',
    'R24': '`if let P = e {` ... `} else {` ... `}` -> `match e { P => {..}, else => {..} }` (single let, optional `&& guard`)',
    'R25': '`for x in &xs` / `xs.iter()` -> `for x in xs`',
    'R26': 'closures `|x| e` -> `fn(x) { e }`; `.any(|x| p)` -> `.filter fn(x) { p }.count() > 0`; `.all(..)` likewise',
    'R27': '`.collect::<T>()` -> `.collect()`',
    'R28': 'path prefixes `crate::ast::`, `ast::`, `crate::types::`, `ty::` dropped',
    'R29': '`.as_ref()`, `.as_slice()`, `.as_str()`, `.as_deref()`, `.borrow()` dropped',
    'R30': '`#[...]` attributes dropped; `pub(crate)`/`pub(super)` -> `pub`/none',
    'R31': 'profile: `self.parsed.text(x)`/`self.text(x)`/`self.name(x)` -> `world.words.resolve(x)`',
    'R32': 'profile: `self.s.f` -> `self.f`, `self.parsed.program.items` -> `world.items`, `self.world.` -> `world.`',
    'R33': 'profile: methods get `world: ref <World>` after `self`; calls `self.m(` to methods in the same input pass `world`',
    'R34': 'profile: `impl<\'a> Checker<\'a>` / `impl Emitter` -> `impl CheckState` / `impl EmitState`',
    'R35': 'postfix `?` in a `T?` function -> `?? return null`',
    'R36': '`loop {` -> `while true {`',
    'R37': 'rustfmt-split patterns and struct literals joined onto one line',
    'R38': '`X::default()` of a moved struct -> `X::empty()`',
    'R39': '`.iter().enumerate()` / slice patterns / `match` on `Some` / `.entry()` / `is_some_and` -> TODO marker',
    'R40': '`.len() as i64` cast dropped, `.unwrap()` -> `?? panic("unwrap")`',
    'R42': 'a function\'s tail expression -> `return expr` (the style of tools/*.nika)',
    'R43': 'zero-argument local closures (`let f = || e;`) inlined at `f()`',
    'R44': '`--file`: a whole tools file, free items on top and `self` methods in `impl <State>`',
    'R45': 'a Rust alias of a moved type (`use nikaia_std::tools::m::{CheckX as X}`) -> its Nikaia name',
    'R46': 'a path call `a::b::f(..)` -> `f(..)` where tools/*.nika has `fn f` (one global namespace)',
    'R47': 'the equivalence table rs2nika.names (`value_node node_key`): Rust helpers that Nikaia has under another name',
    'R48': '`match e { Some(x) => a, None => b }` -> `if e != null { let x = .. a } else { b }`',
    'R49': '`Ty::named(x)` / `Ty::view(x)` -> `Ty::Named { name: x, args: [], view: false/true }`',
    'R50': '`&Span` -> `Span` (a span is copied, never lent)',
    'R51': 'a `fn` nested in a `fn` is hoisted to the top level',
    'R52': 'profile: a `parsed: &Parsed` parameter -> `parsed: ref CheckWorld`, `parsed.text(x)` -> `parsed.words.resolve(x)`',
    'R41': 'function name prefixed (`--prefix`): tools/*.nika is one global namespace',
}

stats = Counter()
NAMES = {'fns': set(), 'types': set(), 'alias': {}, 'equiv': {}}
CTX = {}


def load_names(repo):
    """R45-R47: what the repository already has in Nikaia, read from the tree."""
    import glob
    import os
    for f in glob.glob(os.path.join(repo, 'crates/nikaia-std/src/tools/*.nika')):
        t = open(f, encoding='utf-8').read()
        NAMES['fns'].update(re.findall(r'^(?:pub )?fn (\w+)', t, re.M))
        NAMES['types'].update(re.findall(r'^(?:pub )?(?:struct|enum|trait) (\w+)', t, re.M))
    for f in glob.glob(os.path.join(repo, 'crates/nikaia/src/**/*.rs'), recursive=True):
        t = open(f, encoding='utf-8').read()
        for u in re.findall(r'use nikaia_std::tools::[\w:]*\{[^}]*\}|use nikaia_std::tools::[\w:]+ as \w+', t):
            for a, b in re.findall(r'(\w+) as (\w+)', u):
                NAMES['alias'][b] = a
    eq = os.path.join(repo, 'crates/rs2nika/convert/rs2nika.names')
    if os.path.exists(eq):
        for l in open(eq):
            if l.strip() and not l.startswith('#'):
                a, b = l.split()[:2]
                NAMES['equiv'][a] = b


def hit(rule, n=1):
    stats[rule] += n


# ---------------------------------------------------------------- types

INT = r'\b(?:usize|u32|u16|isize|i32)\b'
COLL = {'HashMap': 'collections::BTreeMap', 'HashSet': 'collections::BTreeSet',
        'BTreeMap': 'collections::BTreeMap', 'BTreeSet': 'collections::BTreeSet'}


def split_generic(s, i):
    """s[i] == '<'; return index of matching '>'."""
    d = 0
    for j in range(i, len(s)):
        if s[j] == '<':
            d += 1
        elif s[j] == '>' and s[j - 1] != '-':
            d -= 1
            if d == 0:
                return j
    return -1


def ty(t):
    """Rewrite a Rust type to Nikaia's spelling."""
    t = t.strip()
    t0 = t
    t = re.sub(r"'\w+\s*,?\s*", '', t) if "'" in t else t
    if t != t0:
        hit('R5')
    t = re.sub(r'\b(?:std::collections::|collections::)', '', t)
    if t.startswith('&mut '):
        return ty(t[5:])
    if t.startswith('&'):
        inner = t[1:].strip()
        if inner == 'Parsed' and CTX.get('world'):
            hit('R52')
            return 'ref ' + CTX['world']
        if inner == 'Span':
            hit('R50')
            return 'Span'
        if inner in ('str', 'String'):
            hit('R4')
            return 'ref String'
        if inner.startswith('[') and inner.endswith(']'):
            hit('R4')
            return 'ref Vec[' + ty(inner[1:-1]) + ']'
        hit('R3')
        return 'ref ' + ty(inner)
    if re.fullmatch(INT, t):
        hit('R10')
        return 'i64'
    if t in ('str',):
        return 'String'
    m = re.match(r'([\w:]+)\s*<', t)
    if m and t.endswith('>'):
        name = m[1]
        inner = t[m.end():-1]
        args = split_args(inner)
        base = name.split('::')[-1]
        if base == 'Option':
            hit('R7')
            return ty(args[0]) + '?'
        if base in ('Box', 'Rc'):
            hit('R9')
            return ty(args[0])
        if base == 'Cow':
            hit('R4')
            return 'String'
        args = ['String' if a.replace(' ', '') in ('&str', "&'astr") else a for a in args]
        if base in COLL:
            hit('R8')
            return COLL[base] + '[' + ', '.join(ty(a) for a in args) + ']'
        hit('R6')
        args = [a for a in args if a]
        if not args:
            return name
        return name.replace('crate::ast::', '').replace('ast::', '') + '[' + ', '.join(ty(a) for a in args) + ']'
    if t.startswith('(') and t.endswith(')'):
        return '(' + ', '.join(ty(a) for a in split_args(t[1:-1])) + ')'
    for p in ('crate::ast::', 'ast::', 'crate::types::', 'ty::'):
        if t.startswith(p):
            hit('R28')
            t = t[len(p):]
    return t


def split_args(s):
    out, d, cur = [], 0, ''
    for ch in s:
        if ch in '<([{':
            d += 1
        elif ch in '>)]}':
            d -= 1
        if ch == ',' and d == 0:
            out.append(cur)
            cur = ''
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return [a.strip() for a in out]


# ---------------------------------------------------------------- signatures

SIG = re.compile(r'^(\s*)(pub(?:\([a-z:]+\))?\s+)?fn\s+(\w+)\s*(<[^(]*>)?\s*\((.*)\)\s*(?:->\s*(.*?))?\s*(\{|where.*)?$')


def signature(line, ctx):
    m = SIG.match(line)
    if not m:
        return None
    ind, pub, name, gen, params, ret, tail = m.groups()
    pub = 'pub ' if pub and pub.strip() in ('pub', 'pub(crate)') else ''
    if m[2] and m[2].strip() != 'pub':
        hit('R30')
    if gen:
        gs = [g for g in split_args(gen[1:-1]) if not g.startswith("'")]
        if len(gs) != len(split_args(gen[1:-1])):
            hit('R5')
        gen = '[' + ', '.join(gs) + ']' if gs else ''
    else:
        gen = ''
    out = []
    has_self = False
    for p in split_args(params):
        p = p.strip()
        if p in ('&self', "&'a self"):
            out.append('ref self'); has_self = True; hit('R2'); continue
        if p == '&mut self':
            out.append('ref mut self'); has_self = True; hit('R2'); continue
        if p in ('self', 'mut self'):
            out.append(p); has_self = True; continue
        if ':' not in p:
            out.append(p); continue
        n, t = p.split(':', 1)
        n, t = n.strip(), t.strip()
        if t.startswith('&mut '):
            hit('R3')
            out.append(f'mut {n}: {ty(t[5:])}')
        elif t.startswith('impl Fn') or t.startswith('&impl Fn') or t.startswith('&mut impl FnMut') or t.startswith('impl FnMut') or t.startswith('&dyn Fn'):
            fm = re.search(r'Fn(?:Mut)?\((.*?)\)\s*(?:->\s*(.*))?$', t)
            a = ', '.join(ty(x) for x in split_args(fm[1])) if fm else ''
            r = (' -> ' + ty(fm[2])) if fm and fm[2] else ''
            out.append(f'{n}: fn({a}){r} sync')
        else:
            out.append(f'{n}: {ty(t)}')
    if has_self and ctx['profile'] != 'none':
        out.insert(1, f"world: ref {ctx['world']}")
        hit('R33')
    r = ''
    throws = ''
    if ret:
        ret = ret.strip()
        rm = re.match(r'(?:anyhow::)?Result<(.*)>$', ret)
        if rm:
            inner = split_args(rm[1])[0]
            r = '' if inner == '()' else ' -> ' + ty(inner)
            throws = ' throws'
            hit('R11')
        else:
            r = ' -> ' + ty(ret)
    if ctx['prefix'] and not name.startswith(ctx['prefix']):
        hit('R41')
        ctx['renamed'][name] = ctx['prefix'] + name
        name = ctx['prefix'] + name
    ctx['fn_returns_opt'] = r.endswith('?')
    hit('R11')
    return f'{ind}{pub}fn {name}{gen}({", ".join(out)}){r} sync{throws} ' + ('{' if tail and tail.startswith('{') else '{')


# ---------------------------------------------------------------- expressions

def closure(s):
    """`|x| expr` and `|x| { ... }` (single line) -> `fn(x) { expr }`."""
    def rep(m):
        args = m[1].replace('&', '').replace('(', '').replace(')', '')
        args = ', '.join(a.split(':')[0].strip() for a in args.split(',') if a.strip())
        return f'fn({args}) {{ '
    out = ''
    i = 0
    while True:
        m = re.search(r'(?:(?<=[(,=])|(?<=[(,=] ))(?:move\s+)?\|([^|]*)\|\s*', s[i:])
        if not m or '||' == s[i + m.start():i + m.start() + 2]:
            out += s[i:]
            break
        a = i + m.start()
        b = i + m.end()
        out += s[i:a]
        # body: until the closing paren of the enclosing call
        d, j = 0, b
        body_block = j < len(s) and s[j] == '{'
        while j < len(s):
            c = s[j]
            if c in '([{':
                d += 1
            elif c in ')]}':
                if d == 0:
                    break
                d -= 1
                if body_block and d == 0:
                    j += 1
                    break
            elif c == ',' and d == 0:
                break
            j += 1
        body = s[b:j].strip()
        if body_block:
            body = body[1:-1].strip()
        out += rep(m) + body + ' }'
        hit('R26')
        i = j
    return out


def iter_adapters(s):
    """R26: `.any(|x| p)` -> `.filter(|x| p).count() > 0`, `.all(|x| p)` ->
    `.filter(|x| !(p)).count() == 0`, `.find(|x| p)` -> `.filter(|x| p).next()`
    (Seq has filter, count and next; closures are rewritten after this)."""
    for name, make in (('any', lambda a, b: f'.filter({a}{b}).count() > 0'),
                       ('all', lambda a, b: f'.filter({a}!({b})).count() == 0'),
                       ('find', lambda a, b: f'.filter({a}{b}).next()')):
        while True:
            m = re.search(r'\.' + name + r'\((\|[^|]*\|\s*)', s)
            if not m:
                break
            d, j = 1, m.end()
            while j < len(s) and d:
                if s[j] in '([{':
                    d += 1
                elif s[j] in ')]}':
                    d -= 1
                j += 1
            body = s[m.end():j - 1].strip()
            if body.startswith('{') and body.endswith('}'):
                body = body[1:-1].strip()
            s = s[:m.start()] + make(m[1], body) + s[j:]
            hit('R26')
    return s


def fmt_macro(s):
    """`format!("..", a, b = c)` -> `f".."`."""
    while True:
        m = re.search(r'&?format!\(\s*"((?:\\.|[^"\\])*)"\s*(,?)', s)
        if not m:
            return s
        # find closing paren
        d, j = 1, m.end()
        while j < len(s) and d:
            if s[j] == '(':
                d += 1
            elif s[j] == ')':
                d -= 1
            j += 1
        args = split_args(s[m.end():j - 1]) if m[2] else []
        text = m[1]
        pos = [a for a in args if not re.match(r'^\w+\s*=[^=]', a)]
        named = dict((a.split('=', 1)[0].strip(), a.split('=', 1)[1].strip()) for a in args if re.match(r'^\w+\s*=[^=]', a))
        k = iter(pos)

        def hole(h):
            inner = h[1]
            nm, _, spec = inner.partition(':')
            if spec and spec != '?':
                return '{' + inner + '}'  # TODO: format specs
            if nm == '':
                v = next(k, '?')
            elif nm.isdigit():
                v = pos[int(nm)] if int(nm) < len(pos) else '?'
            else:
                v = named.get(nm, nm)
            v = v.lstrip('&').lstrip('*')
            return '{' + v + '}'
        text = re.sub(r'(?<!\{)\{([^{}]*)\}(?!\})', hole, text)
        s = s[:m.start()] + 'f"' + text + '"' + s[j:]
        hit('R15')


def matches_macro(s):
    while 'matches!(' in s:
        i = s.index('matches!(')
        d, j = 1, i + 9
        while j < len(s) and d:
            if s[j] in '([{':
                d += 1
            elif s[j] in ')]}':
                d -= 1
            j += 1
        inner = s[i + 9:j - 1]
        parts = split_args(inner)
        if len(parts) < 2:
            break
        e, p = parts[0], ', '.join(parts[1:])
        s = s[:i] + f'(match {e} {{ {p} => true, else => false }})' + s[j:]
        hit('R14')
    return s


def expr(s, ctx):
    o = s
    for p in ('crate::ast::', 'ast::', 'crate::types::', 'crate::'):
        if p in s:
            s = s.replace(p, '')
            hit('R28')
    s = re.sub(r'\bty::(?=[A-Z_a-z])', '', s)
    # profile
    if ctx['profile'] != 'none':
        s2 = re.sub(r'self\.(?:parsed\.)?(?:text|name)\(\*?', 'world.words.resolve(', s)
        s2 = re.sub(r'self\.parsed\.interner\.resolve\(\*?', 'world.words.resolve(', s2)
        s2 = re.sub(r'(?<![\w.])parsed\.(?:text|name)\(\*?', 'parsed.words.resolve(', s2)
        if s2 != s:
            hit('R31')
        s = s2
        s2 = re.sub(r'self\.world\b', 'world', s.replace('self.parsed.program.items', 'world.items').replace('self.s.', 'self.'))
        s2 = re.sub(r'self\.parsed\.', 'world.', s2)
        if s2 != s:
            hit('R32')
        s = s2
        for name in ctx['methods']:
            n2 = ctx['renamed'].get(name, name)
            s3 = re.sub(r'self\.' + name + r'\(\s*', 'self.' + n2 + '(world, ', s)
            s3 = s3.replace('(world, )', '(world)')
            if s3 != s:
                hit('R33')
            s = s3
    for name, n2 in ctx['renamed'].items():
        s = re.sub(r'(?<![.\w])' + name + r'\(', n2 + '(', s)
    s = matches_macro(s)
    s = fmt_macro(s)
    # R45 aliases of moved types, R46 a path call to a function Nikaia has, R47 the equivalence table
    def path_call(m):
        if m[2] in NAMES['fns'] or m[2] in NAMES['equiv']:
            hit('R46')
            return m[2] + '('
        return m[0]
    s = re.sub(r'(?<![\w.])((?:[a-z_]+::)+)([a-z_]\w*)\(', path_call, s)
    for a, b in NAMES['equiv'].items():
        s2 = re.sub(r'(?<![\w.:])' + re.escape(a) + r'\b', b, s)
        if s2 != s:
            hit('R47')
        s = s2
    # constructors
    reps = [
        (r'String::new\(\)', '""', 'R17'), (r'Vec::new\(\)', '[]', 'R17'), (r'vec!\[', '[', 'R17'),
        (r'(?:BTreeMap|HashMap)::(?:new|default)\(\)', 'collections::BTreeMap()', 'R17'),
        (r'(?:BTreeSet|HashSet)::(?:new|default)\(\)', 'collections::BTreeSet()', 'R17'),
        (r'Box::new\(', '(', 'R9'),
        (r'("(?:\\.|[^"\\])*")\.to_string\(\)', r'\1', 'R16'),
        (r'String::from\(("(?:\\.|[^"\\])*")\)', r'\1', 'R16'),
        (r'\.(?:to_owned|into_owned)\(\)', '.clone()', 'R16'),
        (r'(resolve\([^()]*\))\.to_string\(\)', r'\1.clone()', 'R16'),
        (r'\.unwrap_or_else\(\|\|\s*([^()]*(?:\([^()]*\))?)\)', r' ?? \1', 'R20'),
        (r'\.unwrap_or\(([^()]*(?:\([^()]*\))?)\)', r' ?? \1', 'R20'),
        (r'\.cloned\(\)', '?.clone()', 'R20'), (r'\.copied\(\)', '', 'R20'),
        (r'\.is_some\(\)', ' != null', 'R21'), (r'\.is_none\(\)', ' == null', 'R21'),
        (r'\.(?:as_ref|as_slice|as_str|as_deref|borrow|iter)\(\)(?=\s*\{)', '', 'R25'),
        (r'\.(?:as_ref|as_slice|as_str|as_deref|borrow)\(\)', '', 'R29'),
        (r'::<[^()]*>\(\)', '()', 'R27'),
        (r'\bNone\b', 'null', 'R12'),
        (r'(?<=[\s(,=])_ =>', 'else =>', 'R13'), (r'^(\s*)_ =>', r'\1else =>', 'R13'),
        (r'\bloop \{', 'while true {', 'R36'),
        (r'\s+as (?:usize|u32|isize)\b', '', 'R10'),
        (r'\.len\(\) as i64', '.len()', 'R40'),
        (r'\.unwrap\(\)', ' ?? panic("unwrap")', 'R40'),
        (r'\b([A-Z]\w*)::default\(\)', r'\1::empty()', 'R38'),
        (r'\bTy::named\(([^()]*(?:\([^()]*\))?)\)', r'Ty::Named { name: \1, args: [], view: false }', 'R49'),
        (r'\bTy::view\(([^()]*(?:\([^()]*\))?)\)', r'Ty::Named { name: \1, args: [], view: true }', 'R49'),
        (r'\bself\.parsed\b', 'world', 'R32'),
    ]
    for pat, rep, rule in reps:
        s2 = re.sub(pat, rep, s)
        if s2 != s:
            hit(rule)
        s = s2
    s = iter_adapters(s)
    s2 = re.sub(r'(?<=[(,=] )(?:move )?\|\|\s*(.+?)(?=[,)]?$)', r'fn { \1 }', s) if re.search(r'[(,=] (?:move )?\|\|', s) else s
    if s2 != s:
        hit('R26')
    s = closure(s2)
    # Some(x) as a value (not a pattern: patterns end with `=>` or follow `let`/`if let`)
    if not re.search(r'\bif let\b|\blet Some', s):
        head, sep, tail = s.partition('=>') if '=>' in s else ('', '', s)
        s2 = head + sep + re.sub(r'(?<=[=(,:>\[]\s)Some\(|(?<=[=(,:>\[])Some\(|(?<=return )Some\(|^(\s*)Some\(', lambda m: (m[1] or '') + '(', tail)
        if s2 != s:
            hit('R12')
        s = s2
    # derefs and borrows
    s2 = re.sub(r'(?<![\w)\]\s])\*(?=[\w(])', '', s)
    s2 = re.sub(r'(?<=[\s(\[,=!])\*(?=[a-z_(])', '', s2)
    if s2 != s:
        hit('R18')
    s = s2
    s2 = re.sub(r'(?<=[(\[,\s=])&(?:mut\s+)?(?=[\w("\[])', '', s)
    s2 = s2.replace('&&', '&&')
    if s2 != s:
        hit('R19')
    s = s2
    if ctx.get('fn_returns_opt'):
        s2 = re.sub(r'(?<=[\w)\]])\?(?=[.;,)\s]|$)', ' ?? return null', s)
        if s2 != s:
            hit('R35')
        s = s2
    return s


# ---------------------------------------------------------------- statements

def join_split(lines):
    """R37: join rustfmt's multi-line patterns, calls and struct literals."""
    joined = []
    for l in lines:
        if joined and joined[-1].endswith('\\') and joined[-1].count('"') % 2 == 1:
            joined[-1] = joined[-1][:-1] + l.lstrip()
        elif joined and joined[-1].endswith('\\') and '"' in joined[-1]:
            joined[-1] = joined[-1][:-1] + l.lstrip()
        else:
            joined.append(l)
    lines = joined
    out = []
    i = 0
    while i < len(lines):
        l = lines[i]
        st = l.strip()
        if st.startswith('//'):
            out.append(l); i += 1; continue
        code = re.sub(r'"(?:\\.|[^"\\])*"', '""', st)
        opens = code.count('(') + code.count('[') - code.count(')') - code.count(']')
        struct_open = re.search(r'(?:^|[\s(,|])(?:\w+::)*[A-Z]\w*(?:::\w+)* \{$', code) and (
            not re.match(r'^(impl|struct|enum|pub|trait|fn|if|match|while|for|else|loop)\b', code)
            or (re.match(r'^(?:\}\s*else\s+)?(?:if|while) let ', code) and ' = ' not in code)) and not code.startswith('} else')
        if opens > 0 or struct_open:
            buf = l.rstrip()
            d = opens
            sd = 1 if struct_open and opens <= 0 else 0
            j = i + 1
            while j < len(lines) and (d > 0 or sd > 0):
                n = lines[j].strip()
                if n.startswith('//'):
                    out.append(lines[j]); j += 1; continue
                c = re.sub(r'"(?:\\.|[^"\\])*"', '""', n)
                d += c.count('(') + c.count('[') - c.count(')') - c.count(']')
                if sd:
                    for ch in c:
                        if ch == '{':
                            sd += 1
                        elif ch == '}':
                            sd -= 1
                            if sd == 0:
                                break
                sep = '' if buf.endswith(('(', '[', '{')) or n.startswith((')', ']', '.', '?')) else ' '
                if buf.endswith('{') and not n.startswith('}'):
                    sep = ' '
                if n.startswith('}') and not buf.endswith(' '):
                    sep = ' '
                buf += sep + n
                j += 1
                if j - i > 40:
                    break
            buf = re.sub(r',\s*([)\]}])', r' \1', buf).replace(' )', ')').replace(' ]', ']')
            buf = re.sub(r'\{\s+', '{ ', buf)
            out.append(buf)
            hit('R37')
            i = j
            continue
        out.append(l)
        i += 1
    return out


def join_chains(lines):
    out = []
    # multi-line let-else: `let P = e else {` / `return x;` / `};`
    i, tmp = 0, []
    while i < len(lines):
        if re.search(r'\belse \{$', lines[i]) and lines[i].strip().startswith('let ') and i + 2 < len(lines) \
                and lines[i + 2].strip() in ('};', '}') and re.match(r'^\s*(return\b.*|continue|break);?$', lines[i + 1]):
            tmp.append(lines[i] + ' ' + lines[i + 1].strip() + ' }')
            hit('R37')
            i += 3
            continue
        tmp.append(lines[i])
        i += 1
    lines = tmp
    for l in lines:
        st = l.strip()
        prev = out[-1].rstrip() if out else ''
        if out and (st.startswith('.') and not st.startswith('..') or st.startswith('?')) and not prev.lstrip().startswith('//'):
            out[-1] = prev + st
            hit('R37')
        elif out and not prev.lstrip().startswith('//') and (
                (st == '{' and (prev.endswith('=>') or re.match(r'^\s*(\}\s*else\s+)?(if|while|for|match)\b', prev)))
                or st.startswith(('&& ', '|| ', '| '))
                or (st.startswith('if ') and '=>' not in prev and not prev.endswith(('{', ';', ',')) and prev.strip()
                    and len(l) - len(l.lstrip()) > len(prev) - len(prev.lstrip()))):
            out[-1] = prev + ' ' + st
            hit('R37')
        else:
            out.append(l)
    return out


def hoist_nested(lines):
    """R51: a `fn` declared inside a `fn` becomes a top-level `fn` after it."""
    out, moved, i = [], [], 0
    depth = 0
    while i < len(lines):
        l = lines[i]
        if depth > 0 and re.match(r'^\s+fn \w+', l):
            ind = len(l) - len(l.lstrip())
            j = i + 1
            while j < len(lines) and lines[j].rstrip() != ' ' * ind + '}':
                j += 1
            moved.append([x[ind:] if x.startswith(' ' * ind) else x for x in lines[i:j + 1]])
            hit('R51')
            i = j + 1
            continue
        c = re.sub(r'"(?:\\.|[^"\\])*"', '""', l).split('//')[0]
        before = depth
        depth += c.count('{') - c.count('}')
        out.append(l)
        if before > 0 and depth == 0 and moved:
            for m in moved:
                out.append('')
                out += m
            moved = []
        i += 1
    return out


def option_matches(lines):
    """R48: `match e { Some(x) => a, None => b }` -> `if let Some(x) = e { a } else { b }` (then R23)."""
    i = 0
    out = list(lines)
    while i < len(out):
        m = re.match(r'^(\s*)(.*?)\bmatch (.+) \{$', out[i])
        if not m or m[2].strip().startswith('//'):
            i += 1
            continue
        ind = m[1]
        arm = ind + '    '
        j = i + 1
        while j < len(out) and not (out[j].startswith(ind + '}') and len(out[j]) - len(out[j].lstrip()) == len(ind)):
            j += 1
        if j >= len(out):
            i += 1
            continue
        starts = [k for k in range(i + 1, j) if out[k].startswith(arm) and not out[k].startswith(arm + ' ')
                  and not out[k].strip().startswith(('//', '}'))]
        if len(starts) != 2:
            i += 1
            continue
        pats = [re.match(r'^\s*(Some\((\w+)\)|None|_)\s*=>\s*(.*)$', out[k]) for k in starts]
        if not all(pats) or not pats[0][1].startswith('Some'):
            i += 1
            continue
        bodies = []
        for n, k in enumerate(starts):
            end = starts[1] if n == 0 else j
            first = pats[n][3]
            rest = [l for l in out[k + 1:end]]
            if first.startswith('{'):
                body = [first[1:].strip()] + rest
                while body and not body[-1].strip():
                    body.pop()
                if body and body[-1].strip().rstrip(',') == '}':
                    body = body[:-1]
                body = [l for l in body if l.strip()]
            else:
                body = [first.rstrip(',')] + rest
                body[-1] = body[-1].rstrip().rstrip(',')
            bodies.append(body)
        tail = out[j].strip()[1:]
        new = [f'{ind}{m[2]}if let {pats[0][1]} = {m[3]} {{'] + [(arm + b.strip()) if not b.startswith(arm) else b for b in bodies[0]] + \
              [f'{ind}}} else {{'] + [(arm + b.strip()) if not b.startswith(arm) else b for b in bodies[1]] + [f'{ind}}}{tail}']
        out[i:j + 1] = new
        hit('R48')
        i += 1
    return out


def convert(src, profile='check', prefix=''):
    worlds = {'check': ('CheckWorld', 'CheckState'), 'emit': ('EmitWorld', 'EmitState'), 'none': ('', '')}
    CTX['world'] = worlds[profile][0]
    ctx = {'profile': profile, 'world': worlds[profile][0], 'state': worlds[profile][1], 'prefix': prefix,
           'renamed': {}, 'methods': set(), 'fn_returns_opt': False}
    lines = src.split('\n')
    # methods of self in the input
    for l in lines:
        m = SIG.match(l)
        if m and ('self' in (m[5] or '')):
            ctx['methods'].add(m[3])
        if m and prefix and not m[3].startswith(prefix):
            ctx['renamed'][m[3]] = prefix + m[3]
    # R43: zero-argument local closures are inlined where they are called
    inl = {}
    keep = []
    for l in lines:
        m = re.match(r'^\s*let (\w+) = (?:move )?\|\|\s*(.+?);\s*$', l)
        if m:
            inl[m[1]] = m[2]
            hit('R43')
            continue
        keep.append(l)
    lines = keep
    for k, v in inl.items():
        lines = [re.sub(r'(?<![.\w])' + k + r'\(\)', v, l) for l in lines]
    lines = option_matches(join_chains(join_split(join_chains(hoist_nested(lines)))))
    out = []
    # stack of pending closers for `if let` -> match
    stack = []  # (indent, kind)
    inside_impl = False
    for raw in lines:
        st = raw.strip()
        ind = raw[:len(raw) - len(raw.lstrip())]
        if not st:
            out.append(''); continue
        if st.startswith('#['):
            hit('R30'); continue
        if st.startswith('//'):
            out.append(raw); continue
        # impl header
        m = re.match(r'^impl(?:<[^>]*>)?\s+(\w+)(?:<[^>]*>)?\s*\{$', st)
        if m:
            name = m[1]
            if profile != 'none' and name in ('Checker', 'Emitter'):
                hit('R34')
                name = ctx['state']
            out.append(f'{ind}impl {name} {{')
            inside_impl = True
            continue
        # struct/enum fields
        m = re.match(r'^(pub(?:\([a-z]+\))?\s+)?(struct|enum)\s+(\w+)(<[^>]*>)?\s*\{$', st)
        if m:
            out.append(f'{ind}{"pub " if m[1] else ""}{m[2]} {m[3]} {{'); continue
        m = re.match(r'^(pub(?:\([a-z]+\))?\s+)?(\w+):\s*(.+?),$', st)
        if m and not stack and not re.search(r'=>|\(', st.split(':', 1)[0]) and len(out) and out_kind(out) == 'struct':
            out.append(f'{ind}{"pub " if m[1] else ""}{m[2]}: {ty(m[3])},'); continue
        sig = signature(raw, ctx)
        if sig:
            out.append(sig); continue
        line = raw
        todo = []
        # R22: let-else
        m = re.match(r'^(\s*)let\s+Some\((\w+)\)\s*=\s*(.+?)\s+else\s*\{\s*(return\b[^}]*|continue|break)\s*;?\s*\}\s*;?$', line)
        if m:
            line = f'{m[1]}let {m[2]} = {m[3]} ?? {m[4].strip().rstrip(";")}'
            hit('R22')
        else:
            m = re.match(r'^(\s*)let\s+(.+?)\s*=\s*(.+?)\s+else\s*\{\s*(return\b[^}]*|continue|break)\s*;?\s*\}\s*;?$', line)
            if m:
                pat = m[2]
                names = [n for n in re.findall(r'(?<![\w:])([a-z_]\w*)\b(?!\s*:)(?!::)', re.sub(r'"[^"]*"', '', pat)) if n not in ('ref', 'mut')]
                if len(names) == 1:
                    line = f'{m[1]}let {names[0]} = match {m[3]} {{ {pat} => {names[0]}, else => {m[4].strip().rstrip(";")} }}'
                    hit('R22')
                else:
                    todo.append('let-else binding several names: write a `match`')
        # R23 / R24: if let
        m = re.match(r'^(\s*)((?:let\s+(?:mut\s+)?\w+(?:\s*:[^=]+)?\s*=\s*|return\s+|[\w.]+\s*=\s*)?)(\}\s*else\s+)?if let\s+(.+?)\s*=\s*(.+?)\s*\{$', line)
        pre = ''
        if m:
            pre = expr(m[2], ctx) if m[2] else ''
            m = [m[0], m[1], m[3], m[4], m[5]]
            if pre and not re.fullmatch(r'Some\((\w+)\)', m[3]):
                m = None
                todo.append('`if let` with a pattern as a value: write a `match`')
        if m and ' && let ' not in line:
            pat, val = m[3], m[4]
            if pat == 'Some(_)':
                out.append(f'{m[1]}{pre}if {expr(val, ctx)} != null {{')
                hit('R23')
                stack.append((m[1], 'if'))
                continue
            guard = ''
            if ' && ' in val:
                val, guard = val.split(' && ', 1)
            pm = re.fullmatch(r'Some\((\w+)\)', pat)
            if m[2]:
                todo.append('`else if let`: nest it inside the `else`')
            elif pm and not guard:
                v = expr(val, ctx)
                out.append(f'{m[1]}{pre}if {v} != null {{')
                out.append(f'{m[1]}    let {pm[1]} = {v} ?? panic("rs2nika: checked above")')
                hit('R23')
                stack.append((m[1], 'if'))
                continue
            elif not pm:
                v = expr(val, ctx)
                g = (' if ' + expr(guard, ctx)) if guard else ''
                out.append(f'{m[1]}match {v} {{')
                out.append(f'{m[1]}    {expr(pat, ctx)}{g} => {{')
                hit('R24')
                stack.append((m[1], 'iflet'))
                continue
            else:
                todo.append('`if let Some(..) = e && guard`: write `let x = e ?? ..` then `if`')
        if ' && let ' in line or re.match(r'^\s*(?:\}\s*else\s+)?if let .*&&\s*$', line) or re.match(r'^\s*&& let ', line):
            todo.append('let-chain (`&& let`): nest `match`es')
        # closing of an `if let` turned `match`
        if stack and st.startswith('}') and ind == stack[-1][0]:
            kind = stack[-1][1]
            if kind == 'iflet':
                if re.match(r'^\}\s*else\s*\{$', st):
                    out.append(f'{ind}    }},')
                    out.append(f'{ind}    else => {{')
                    stack[-1] = (ind, 'iflet-else')
                    continue
                if re.match(r'^\}\s*else if', st):
                    out.append(f'{ind}    }},')
                    out.append(f'{ind}    // TODO(rs2nika): `else if` after `if let`')
                    out.append(f'{ind}    else => {{ if {expr(st.split("else if", 1)[1].rstrip("{").strip(), ctx)} {{')
                    stack[-1] = (ind, 'iflet-elseif')
                    continue
                out.append(f'{ind}    }},')
                out.append(f'{ind}    else => {{}},')
                out.append(f'{ind}}}' + st[1:].rstrip(';'))
                stack.pop()
                continue
            if kind == 'iflet-else':
                if st.startswith('} else'):
                    pass
                else:
                    out.append(f'{ind}    }},')
                    out.append(f'{ind}}}' + st[1:].rstrip(';'))
                    stack.pop()
                    continue
            if kind == 'iflet-elseif' and not st.startswith('} else'):
                out.append(f'{ind}    }} }},')
                out.append(f'{ind}}}' + st[1:].rstrip(';'))
                stack.pop()
                continue
            if kind == 'if' and not st.startswith('} else'):
                stack.pop()
        if stack and stack[-1][1] in ('iflet', 'iflet-else', 'iflet-elseif'):
            line = '    ' + line
        # enumerate, slice patterns, Some patterns, entry
        if re.match(r'^\s*static ', line):
            todo.append('`static`: no globals in tools/*.nika; a field of the state struct')
        if re.search(r"\bb'", line):
            todo.append("byte literal `b'x'`: its number")
        if '.and_then(' in line or '.map(|' in line and re.search(r'\?|unwrap_or|\.map\(\|[^|]*\|[^)]*\)\s*$', line) and 'iter()' not in line:
            todo.append('`Option::map`/`and_then`: `let x = e ?? <none>` then the body')
        if re.search(r'->\s*Option<\(', line):
            todo.append('`(A, B)?` is not written (ADR-278 D1): a named struct')
        if '.enumerate()' in line:
            todo.append('`.enumerate()`: a `while` with an index')
        if re.search(r'(?:^|[\s(|])Some\(.*=>', line) or re.search(r'^\s*None\s*=>', line):
            todo.append('`match` on `Some`/`None`: `let x = e ?? <none arm>`')
        if ('=>' in line and re.search(r'[(,]\s*\[(?:\s*(?:\w+|\.\.|_)\s*,?)*\]', line.split('=>')[0])) or re.search(r'let \[', line) or re.search(r'\[\s*\w+\s*,\s*(?:\w+|\.\.)\s*\]\s*(?:=>|=)', line):
            todo.append('slice pattern: index by position after a `len()` check')
        if '.entry(' in line:
            todo.append('`.entry(..)`: read with `??`, then write `m[k] = v`')
        if 'is_some_and' in line or 'is_ok_and' in line or 'map_or' in line:
            todo.append('`is_some_and`/`map_or`: `let x = e ?? <false>` then the test')
        if '.all_rs2nika(' in line:
            line = line.replace('.all_rs2nika(', '.all(')
            todo.append('`.all(p)`: `.filter fn(x) { !(p) }.count() == 0` or a `for` loop')
        if re.search(r'\.(?:find|position|find_map|filter_map|flat_map|any|fold|take_while|skip|last|max|min|sum)\(', line):
            todo.append('iterator adapter with no `Seq` method: a `for` loop')
        if re.search(r"'\w+\b(?!')", line) and '<' in line and '&' in line:
            pass
        lm = re.match(r'^(\s*let\s+(?:mut\s+)?\w+\s*:\s*)(.+?)(\s*=\s.*)$', line)
        if lm and '<' in lm[2] or (lm and '&' in lm[2]):
            line = lm[1] + ty(lm[2]) + lm[3]
        line = expr(line, ctx)
        # R1 semicolons
        code = line.rstrip()
        if code.endswith(';') and not code.lstrip().startswith('//'):
            code = code[:-1]
            hit('R1')
        code = code.replace('};', '}')
        for t in todo:
            out.append(f'{ind}// TODO(rs2nika): {t}')
        out.append(code)
    text = '\n'.join(tail_returns(out))
    for y, x in NAMES['alias'].items():
        if y[:1].isupper() and x[:1].isupper():
            t2 = re.sub(r'(?<![\w:.])' + y + r'\b', x, text)
            if t2 != text:
                hit('R45')
            text = t2
    return text


def tail_returns(out):
    """R42: a function's last expression becomes `return expr` (the tools' style)."""
    i = 0
    while i < len(out):
        m = re.match(r'^(\s*)(?:pub )?fn .*->.*\{$', out[i])
        if not m:
            i += 1
            continue
        ind = m[1]
        j = i + 1
        while j < len(out) and out[j].rstrip() != ind + '}':
            j += 1
        body = ind + '    '
        k = j - 1
        while k > i and (not out[k].strip() or out[k].strip().startswith('//')):
            k -= 1
        start = k

        def at_body(l):
            return l.startswith(body) and not l.startswith(body + ' ') and not l.strip().startswith(('}', '//'))
        while start > i and not at_body(out[start]):
            start -= 1
        st = out[start].strip()
        if start > i and not re.match(r'^(return|let|for|while|if|//|\}|else|[a-z_.]+\s*(\+|-)?=[^=>])', st) and not st.startswith('match ') or (start > i and st.startswith('match ')):
            out[start] = body + 'return ' + st
            hit('R42')
        i = j + 1
    return out


def out_kind(out):
    for l in reversed(out):
        s = l.strip()
        if re.match(r'^(pub )?struct \w+ \{$', s):
            return 'struct'
        if s.endswith('{') or s.startswith('}'):
            return 'other'
    return 'other'


def items(text):
    """Split a draft into top-level items by brace depth: (kind, lines)."""
    out, cur, depth, pending = [], None, 0, []
    for l in text.split('\n'):
        st = l.strip()
        if depth == 0:
            if re.match(r'^(?:pub )?(?:fn|struct|enum|impl)\b', st):
                kind = 'method' if re.search(r'\((?:ref (?:mut )?)?self\b', l) else 'item'
                cur = (kind, pending + [l])
                out.append(cur)
                pending = []
            elif st.startswith('//'):
                pending.append(l)
                continue
            else:
                continue
        else:
            cur[1].append(l)
        c = re.sub(r'"(?:\\.|[^"\\])*"', '""', l).split('//')[0]
        depth += c.count('{') - c.count('}')
        depth = max(depth, 0)
    res = []
    for kind, ls in out:
        sig = [x for x in ls if not x.strip().startswith('//')][0]
        d = len(sig) - len(sig.lstrip())
        res.append((kind, [x[d:] if x.startswith(' ' * d) else x.lstrip() for x in ls]))
    return res


def as_file(text, state):
    """R44: a whole tools file: `use std::collections`, free items at the top level,
    methods of `self` inside `impl <State> { }`."""
    its = items(text)
    free = [ls for k, ls in its if k == 'item']
    meth = [ls for k, ls in its if k == 'method']
    out = ['use std::collections', '']
    for ls in free:
        out += ls + ['']
    if meth:
        out.append(f'impl {state} {{')
        for ls in meth:
            out += ['    ' + x if x.strip() else '' for x in ls] + ['']
        out[-1] = '}'
    return '\n'.join(out) + '\n'


LINTS = [
    (r'\.get\([^()]*(?:\([^()]*\))?[^()]*\)\?\.clone\(\)', 'D2: `m.get(k)?.clone()` lowers to a clone of a reference; write `m[k]?.clone()`'),
    (r'\.len\(\) as i64', 'D7: `.len()` is already `i64`; the cast is doubled (clippy unnecessary_cast)'),
    (r'\bget\([^)]*\bas i64\)|\[[^\]]*\bas i64\]', 'D8: a cast inside a map read lowers as `&x as i64`; bind `let k = x as i64` first'),
    (r'=>\s*[\w.]+\.insert\(', 'D3: `insert` as an arm value yields `bool` against `()`; write `=> { let _added = s.insert(..) }`'),
    (r'\b(?!self\b)(?!clone\(\))\w+ with \{', 'D1: `p with {..}` on a by-value parameter writes `..p` of a `&T`; on a parameter write the struct literal in full'),
    (r'\.to_owned\(\)', 'NK1189: `.to_owned()` is `.clone()`'),
    (r'\bif let\b', 'no `if let`: `x ?? return`, or `if x != null` then `x ?? panic(..)`'),
    (r'(?<![\w)\]\s])\*[a-z_]', 'no `*x`'),
    (r'\bif match\b', 'a `match` in a guard or condition needs parentheses: `if (match ..)`'),
    (r'\)\?\s+(?:sync|\{|throws)', '`(A, B)?` is not written (ADR-278 D1): a named struct'),
]


def lint(path):
    """`--lint FILE.nika`: the known lowering defects and syntax gaps, before a 40 s lower-std."""
    n, params, in_str = 0, set(), False
    for i, l in enumerate(open(path, encoding='utf-8'), 1):
        was = in_str
        if (len(re.findall(r'(?<!\\)"', l)) % 2) == 1:
            in_str = not in_str
        if was or l.strip().startswith('//'):
            continue
        sig = re.match(r'\s*(?:pub )?fn \w+(?:\[[^\]]*\])?\((.*)\)', l)
        if sig:
            params = {m[1] for m in re.finditer(r'(?:^|,\s*)(?:mut )?(\w+): (?!ref\b)', sig[1])}
        code = re.sub(r'f?"(?:\\.|[^"\\])*"', '""', l)
        for pat, msg in LINTS:
            if msg.startswith('D1:'):
                m = re.search(r'\b(\w+) with \{', code)
                if not (m and m[1] in params):
                    continue
            if re.search(pat, code):
                print(f'{path}:{i}: {msg}')
                n += 1
    return n


def subprocess_root():
    import subprocess
    r = subprocess.run(['git', 'rev-parse', '--show-toplevel'], capture_output=True, text=True)
    return r.stdout.strip() if r.returncode == 0 else ''


def main():
    a = sys.argv[1:]
    if '--lint' in a:
        i = a.index('--lint')
        sys.exit(1 if sum(lint(f) for f in a[i + 1:]) else 0)
    if '--rules' in a:
        for k, v in RULES.items():
            print(f'{k:4} {v}')
        return
    profile, prefix, show = 'check', '', False
    if '--profile' in a:
        i = a.index('--profile'); profile = a[i + 1]; del a[i:i + 2]
    if '--prefix' in a:
        i = a.index('--prefix'); prefix = a[i + 1]; del a[i:i + 2]
    if '--stats' in a:
        a.remove('--stats'); show = True
    repo = subprocess_root()
    if '--repo' in a:
        i = a.index('--repo'); repo = a[i + 1]; del a[i:i + 2]
    if repo:
        load_names(repo)
    whole = '--file' in a
    if whole:
        a.remove('--file')
    src = sys.stdin.read() if not a or a[0] == '-' else open(a[0]).read()
    d = convert(src, profile, prefix)
    print(as_file(d, {'check': 'CheckState', 'emit': 'EmitState'}.get(profile, 'State')) if whole else d)
    if NAMES['fns']:
        own = set(re.findall(r'\bfn (\w+)', d))
        called = set(re.findall(r'(?<![\w.:])([a-z_]\w*)\(', re.sub(r'"(?:\\.|[^"\\])*"', '""', d)))
        kw = {'fn', 'if', 'match', 'while', 'for', 'return', 'panic', 'println', 'sync', 'f', 'in', 'and', 'or', 'not'}
        missing = sorted(called - own - NAMES['fns'] - kw)
        if missing:
            print('// rs2nika: called but not in Nikaia yet (move them first, or name the equivalent in rs2nika.names): '
                  + ', '.join(missing), file=sys.stderr)
    if show:
        for k, v in sorted(stats.items(), key=lambda kv: -kv[1]):
            print(f'{k:4} {v:5}  {RULES[k]}', file=sys.stderr)


if __name__ == '__main__':
    main()
