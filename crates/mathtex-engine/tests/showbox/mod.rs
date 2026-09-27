//! `\showbox` parsing, TeX's child placement, and the comparison against lowered IR.

use mathtex_ir::{BoxKind, Fragment, LayoutNodeKind, NodeId};

/// TeX's `null_flag`, `\showbox` prints running rule dimensions as `*`.
pub const RUNNING_DIM: i32 = -1_073_741_824;
/// TeX's `billion`, the bound `vet_glue` clamps glue products to.
const BILLION: f64 = 1_000_000_000.0;

/// A normalized layout node, dimensions in scaled points.
#[derive(Clone, Debug, PartialEq)]
pub enum Norm {
    Box {
        kind: BoxKind,
        width: i32,
        height: i32,
        depth: i32,
        /// Shift and glue setting, only the `\showbox` side knows them.
        shift: i32,
        set: GlueSet,
        children: Vec<Child>,
    },
    Rule {
        width: i32,
        height: i32,
        depth: i32,
    },
    /// Set size, `\showbox` gives the natural size and [`place`] applies the glue setting.
    Glue {
        amount: i32,
        stretch: (i32, i32),
        shrink: (i32, i32),
    },
    Kern {
        amount: i32,
    },
    /// Glyph code, `\showbox` prints no metrics so [`reconcile_widths`] copies the engine's.
    Glyph {
        glyph: u32,
        width: i32,
        height: i32,
        depth: i32,
    },
    /// A line with no geometry, dropped while parsing.
    Skip,
}

/// Glue setting of a box: sign 0 normal, 1 stretching, 2 shrinking.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GlueSet {
    sign: i32,
    order: i32,
    ratio: f64,
}

/// A child with its reference point relative to the parent's, y down, and the slack the oracle allows.
#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    pub at: (i32, i32),
    /// `\showbox` rounds the glue set ratio, so positions after stretched glue carry a small slack.
    pub slack: i32,
    pub node: Norm,
}

impl Norm {
    pub fn label(&self) -> &'static str {
        match self {
            Norm::Box { .. } => "box",
            Norm::Rule { .. } => "rule",
            Norm::Glue { .. } => "glue",
            Norm::Kern { .. } => "kern",
            Norm::Glyph { .. } => "glyph",
            Norm::Skip => "skip",
        }
    }
}

/// Extracts the `\showbox0` dump from a TeX log.
pub fn extract_showbox(log: &str) -> Vec<String> {
    let mut lines = log.lines();
    for line in lines.by_ref() {
        if line.starts_with("> \\box0=") {
            break;
        }
    }
    lines
        .take_while(|line| !line.trim().is_empty() && !line.starts_with('!'))
        .map(str::to_string)
        .collect()
}

/// Converts a `\showbox` decimal back to exact scaled points, as TeX's `round_decimals`.
pub fn pt_to_sp(text: &str) -> i32 {
    let text = text.trim();
    let negative = text.starts_with('-');
    let text = text.trim_start_matches(['+', '-']);
    let (int_part, frac_part) = text.split_once('.').unwrap_or((text, ""));
    let int_val: i64 = int_part.parse().unwrap_or(0);
    let digits = frac_part
        .bytes()
        .take_while(u8::is_ascii_digit)
        .take(17)
        .collect::<Vec<_>>();
    let mut a: i64 = 0;
    for d in digits.into_iter().rev() {
        a = (a + i64::from(d - b'0') * 131_072) / 10;
    }
    let sp = int_val * 65_536 + (a + 1) / 2;
    (if negative { -sp } else { sp }) as i32
}

/// Parses a `\showbox` dump and places every child as `hlist_out` and `vlist_out` would.
pub fn parse_showbox(block: &[String]) -> Norm {
    let lines: Vec<(usize, &str)> = block
        .iter()
        .map(|line| {
            let depth = line.chars().take_while(|c| *c == '.').count();
            (depth, &line[depth..])
        })
        .collect();
    let mut index = 0;
    parse_node(&lines, &mut index, 0)
}

fn parse_node(lines: &[(usize, &str)], index: &mut usize, depth: usize) -> Norm {
    let text = lines[*index].1;
    *index += 1;
    let mut node = parse_line(text);
    if let Norm::Box { children, .. } = &mut node {
        while *index < lines.len() && lines[*index].0 > depth {
            // Deeper lines without a child header of their own, such as ligature parts, are skipped.
            if lines[*index].0 != depth + 1 {
                *index += 1;
                continue;
            }
            let child = parse_node(lines, index, depth + 1);
            if child != Norm::Skip {
                children.push(Child {
                    at: (0, 0),
                    slack: 0,
                    node: child,
                });
            }
        }
    }
    node
}

fn parse_line(text: &str) -> Norm {
    let text = text.trim();
    for (prefix, kind) in [
        ("\\hbox(", BoxKind::Horizontal),
        ("\\vbox(", BoxKind::Vertical),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            let (width, height, depth) = parse_dims(rest);
            return Norm::Box {
                kind,
                width,
                height,
                depth,
                shift: field(rest, ", shifted ").map_or(0, pt_to_sp),
                set: parse_glue_set(rest),
                children: Vec::new(),
            };
        }
    }
    if let Some(rest) = text.strip_prefix("\\rule(") {
        let (width, height, depth) = parse_dims(rest);
        return Norm::Rule {
            width,
            height,
            depth,
        };
    }
    if let Some(rest) = text.strip_prefix("\\kern") {
        // Font kerns print `\kern0.16`, explicit kerns `\kern 3.0`, accent kerns add `(for accent)`.
        let amount = rest.split_whitespace().next().unwrap_or("0");
        return Norm::Kern {
            amount: pt_to_sp(amount),
        };
    }
    if let Some(rest) = text.strip_prefix("\\glue") {
        let rest = rest.trim_start();
        let rest = match rest.strip_prefix('(') {
            Some(inner) => inner.split_once(')').map_or(inner, |(_, after)| after),
            None => rest,
        };
        let mut words = rest.split_whitespace();
        let amount = pt_to_sp(words.next().unwrap_or("0"));
        let (mut stretch, mut shrink) = ((0, 0), (0, 0));
        while let Some(word) = words.next() {
            let value = words.next().map_or((0, 0), parse_order);
            match word {
                "plus" => stretch = value,
                "minus" => shrink = value,
                _ => {}
            }
        }
        return Norm::Glue {
            amount,
            stretch,
            shrink,
        };
    }
    if let Some(rest) = text
        .strip_prefix("\\mathon")
        .or_else(|| text.strip_prefix("\\mathoff"))
    {
        // A math node moves by `\mathsurround`, the engine lowers a nonzero surround as a kern.
        return field(rest, ", surrounded ").map_or(Norm::Skip, |surround| Norm::Kern {
            amount: pt_to_sp(surround),
        });
    }
    if text.starts_with("\\penalty") {
        return Norm::Skip;
    }
    if let Some(glyph) = parse_glyph(text) {
        return Norm::Glyph {
            glyph,
            width: i32::MIN,
            height: 0,
            depth: 0,
        };
    }
    // Unknown lines count as a zero kern so structural differences surface in the child count.
    Norm::Kern { amount: 0 }
}

/// Reads `\font A` for TFM characters and `\font glyph#89` for native glyphs.
fn parse_glyph(text: &str) -> Option<u32> {
    let (_font, rest) = text.strip_prefix('\\')?.split_once(' ')?;
    if let Some(number) = rest.strip_prefix("glyph#") {
        let end = number
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(number.len());
        return number[..end].parse().ok();
    }
    rest.chars().next().map(u32::from)
}

/// The value after `key` up to the next comma.
fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let start = text.find(key)? + key.len();
    Some(text[start..].split(',').next().unwrap_or("").trim())
}

/// Parses `1.0fil` into scaled points and an infinity order.
fn parse_order(text: &str) -> (i32, i32) {
    let digits = text.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    let order = match &text[digits.len()..] {
        "fil" => 1,
        "fill" => 2,
        "filll" => 3,
        _ => 0,
    };
    (pt_to_sp(digits), order)
}

/// Parses `, glue set 0.33333fil` or `, glue set - 0.5`.
fn parse_glue_set(text: &str) -> GlueSet {
    let Some(value) = field(text, ", glue set ") else {
        return GlueSet::default();
    };
    let (sign, value) = match value.strip_prefix("- ") {
        Some(value) => (2, value),
        None => (1, value),
    };
    let (ratio, order) = parse_order(value);
    GlueSet {
        sign,
        order,
        ratio: f64::from(ratio) / 65_536.0,
    }
}

fn parse_dims(rest: &str) -> (i32, i32, i32) {
    let close = rest.find(')').unwrap_or(rest.len());
    let (height, depth) = rest[..close]
        .split_once('+')
        .unwrap_or((&rest[..close], "0.0"));
    let after = &rest[close..];
    let width = after.find('x').map_or(0, |at| {
        let text = &after[at + 1..];
        let end = text
            .find(|c: char| c == ',' || c.is_whitespace())
            .unwrap_or(text.len());
        parse_running(&text[..end])
    });
    (width, parse_running(height), parse_running(depth))
}

fn parse_running(text: &str) -> i32 {
    match text.trim() {
        "*" => RUNNING_DIM,
        text => pt_to_sp(text),
    }
}

/// Copies engine glyph metrics into the oracle, which `\showbox` leaves without them.
pub fn reconcile_widths(expected: &mut Norm, actual: &Norm) {
    match (expected, actual) {
        (Norm::Box { children: ec, .. }, Norm::Box { children: ac, .. }) => {
            for (ce, ca) in ec.iter_mut().zip(ac) {
                reconcile_widths(&mut ce.node, &ca.node);
            }
        }
        (
            Norm::Glyph {
                width: ew,
                height: eh,
                depth: ed,
                ..
            },
            Norm::Glyph {
                width: aw,
                height: ah,
                depth: ad,
                ..
            },
        ) if *ew == i32::MIN => (*ew, *eh, *ed) = (*aw, *ah, *ad),
        _ => {}
    }
}

/// Resolves running rule dimensions and places each child as TeX's `hlist_out` and `vlist_out` do.
pub fn place(node: &mut Norm) {
    let Norm::Box {
        kind,
        width,
        height,
        depth,
        set,
        children,
        ..
    } = node
    else {
        return;
    };
    let vertical = *kind == BoxKind::Vertical;
    let mut at = if vertical { -*height } else { 0 };
    let (mut cur_g, mut cur_glue) = (0i32, 0f64);
    // An error of 2^-17 in the printed ratio moves a glue by at most |cur_glue| * 2^-17 plus a rounding flip.
    let slack = |cur_glue: f64| {
        if set.sign == 0 {
            0
        } else {
            2 * ((cur_glue.abs() / 131_072.0).ceil() as i32 + 1)
        }
    };
    for child in children.iter_mut() {
        child.slack = slack(cur_glue);
        let point = |cross: i32, along: i32| {
            if vertical {
                (cross, along)
            } else {
                (along, cross)
            }
        };
        match &mut child.node {
            Norm::Box {
                width: w,
                height: h,
                depth: d,
                shift,
                ..
            } => {
                if vertical {
                    at += *h;
                    child.at = point(*shift, at);
                    at += *d;
                } else {
                    child.at = point(*shift, at);
                    at += *w;
                }
                place(&mut child.node);
            }
            Norm::Rule {
                width: w,
                height: h,
                depth: d,
            } => {
                let running = |value: &mut i32, fallback: i32| {
                    if *value == RUNNING_DIM {
                        *value = fallback;
                    }
                };
                if vertical {
                    running(w, *width);
                    child.at = point(0, at + *h);
                    at += *h + *d;
                } else {
                    running(h, *height);
                    running(d, *depth);
                    child.at = point(0, at);
                    at += *w;
                }
            }
            Norm::Glue {
                amount,
                stretch,
                shrink,
            } => {
                child.at = point(0, at);
                let mut set_amount = *amount - cur_g;
                let delta = match set.sign {
                    1 if stretch.1 == set.order => Some(f64::from(stretch.0)),
                    2 if shrink.1 == set.order => Some(-f64::from(shrink.0)),
                    _ => None,
                };
                if let Some(delta) = delta {
                    cur_glue += delta;
                    cur_g = (set.ratio * cur_glue).clamp(-BILLION, BILLION).round() as i32;
                }
                set_amount += cur_g;
                *amount = set_amount;
                *stretch = (0, 0);
                *shrink = (0, 0);
                child.slack = slack(cur_glue);
                at += set_amount;
            }
            Norm::Kern { amount } => {
                child.at = point(0, at);
                at += *amount;
            }
            Norm::Glyph {
                width: w,
                height: h,
                depth: d,
                ..
            } => {
                // A glyph the engine side has no counterpart for keeps no width.
                let w = if *w == i32::MIN { 0 } else { *w };
                if vertical {
                    // XeTeX's vlist_out stacks a glyph node by its height and depth.
                    at += *h;
                    child.at = point(0, at);
                    at += *d;
                } else {
                    child.at = point(0, at);
                    at += w;
                }
            }
            Norm::Skip => {}
        }
    }
}

/// Normalizes the lowered fragment's root with every child at its IR origin.
pub fn normalize_fragment(fragment: &Fragment) -> Result<Norm, String> {
    fragment.validate().map_err(|error| error.to_string())?;
    Ok(normalize(fragment, fragment.root))
}

fn normalize(fragment: &Fragment, id: NodeId) -> Norm {
    let node = fragment.node(id).expect("node id should resolve");
    let (width, height, depth) = (node.width.0, node.height.0, node.depth.0);
    match &node.kind {
        LayoutNodeKind::Box(layout_box) => Norm::Box {
            kind: layout_box.kind,
            width,
            height,
            depth,
            shift: 0,
            set: GlueSet::default(),
            children: layout_box
                .children
                .iter()
                .map(|&child| {
                    let child_node = fragment.node(child).expect("child");
                    Child {
                        at: (child_node.origin.x.0, child_node.origin.y.0),
                        slack: 0,
                        node: normalize(fragment, child),
                    }
                })
                .collect(),
        },
        LayoutNodeKind::Rule => Norm::Rule {
            width,
            height,
            depth,
        },
        LayoutNodeKind::Glue(glue) => Norm::Glue {
            amount: glue.amount.0,
            stretch: (0, 0),
            shrink: (0, 0),
        },
        LayoutNodeKind::Kern(kern) => Norm::Kern {
            amount: kern.amount.0,
        },
        LayoutNodeKind::GlyphRun(run) => Norm::Glyph {
            glyph: run.glyphs.first().map_or(0, |glyph| glyph.glyph_id.0),
            width,
            height,
            depth,
        },
        _ => Norm::Skip,
    }
}

/// Compares the oracle with the engine, dimensions exactly and positions within the oracle's slack.
pub fn diff(expected: &Norm, actual: &Norm, path: &str, slack: i32, out: &mut Vec<String>) {
    let dim = |out: &mut Vec<String>, what: &str, e: i32, a: i32, slack: i32| {
        if (i64::from(e) - i64::from(a)).abs() > i64::from(slack) {
            out.push(format!(
                "{path}: {what} tex={e}sp ({:.5}pt) engine={a}sp ({:.5}pt)",
                f64::from(e) / 65536.0,
                f64::from(a) / 65536.0,
            ));
        }
    };
    match (expected, actual) {
        (
            Norm::Box {
                kind: ek,
                width: ew,
                height: eh,
                depth: ed,
                children: ec,
                ..
            },
            Norm::Box {
                kind: ak,
                width: aw,
                height: ah,
                depth: ad,
                children: ac,
                ..
            },
        ) => {
            dim(out, "width", *ew, *aw, 0);
            dim(out, "height", *eh, *ah, 0);
            dim(out, "depth", *ed, *ad, 0);
            if ek != ak {
                out.push(format!("{path}: box kind tex={ek:?} engine={ak:?}"));
            }
            if ec.len() != ac.len() {
                let labels = |children: &[Child]| {
                    children
                        .iter()
                        .map(|child| child.node.label())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                out.push(format!(
                    "{path}: child count tex={} engine={} (tex [{}], engine [{}])",
                    ec.len(),
                    ac.len(),
                    labels(ec),
                    labels(ac),
                ));
            }
            for (i, (ce, ca)) in ec.iter().zip(ac).enumerate() {
                let child_path = format!("{path}.{i}");
                for (axis, e, a) in [("x", ce.at.0, ca.at.0), ("y", ce.at.1, ca.at.1)] {
                    if (i64::from(e) - i64::from(a)).abs() > i64::from(ce.slack) {
                        out.push(format!(
                            "{child_path}: origin {axis} tex={e}sp engine={a}sp"
                        ));
                    }
                }
                diff(&ce.node, &ca.node, &child_path, ce.slack, out);
            }
        }
        (
            Norm::Rule {
                width: ew,
                height: eh,
                depth: ed,
            },
            Norm::Rule {
                width: aw,
                height: ah,
                depth: ad,
            },
        ) => {
            dim(out, "rule.width", *ew, *aw, 0);
            dim(out, "rule.height", *eh, *ah, 0);
            dim(out, "rule.depth", *ed, *ad, 0);
        }
        (Norm::Glue { amount: e, .. }, Norm::Glue { amount: a, .. }) => {
            dim(out, "glue", *e, *a, slack)
        }
        (Norm::Kern { amount: e }, Norm::Kern { amount: a }) => dim(out, "kern", *e, *a, 0),
        (
            Norm::Glyph {
                glyph: eg,
                width: ew,
                ..
            },
            Norm::Glyph {
                glyph: ag,
                width: aw,
                ..
            },
        ) => {
            if eg != ag {
                out.push(format!("{path}: glyph tex={eg} engine={ag}"));
            }
            dim(out, "glyph.width", *ew, *aw, 0);
        }
        (e, a) => out.push(format!(
            "{path}: node kind tex={} engine={}",
            e.label(),
            a.label()
        )),
    }
}

/// Parses and places a dump, reconciles glyph widths from the engine, and lists every difference.
pub fn compare(block: &[String], actual: &Result<Norm, String>, name: &str) -> (Norm, Vec<String>) {
    let mut expected = parse_showbox(block);
    let mut diffs = Vec::new();
    match actual {
        Ok(actual) => {
            reconcile_widths(&mut expected, actual);
            place(&mut expected);
            diff(&expected, actual, name, 0, &mut diffs);
        }
        Err(reason) => diffs.push(format!("{name}: engine produced no layout ({reason})")),
    }
    (expected, diffs)
}
