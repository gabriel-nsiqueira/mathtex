//! `syn` AST passes over translated method bodies.

use std::collections::BTreeSet;

use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, parse_str, Expr, ExprCall, Local, Type};

use crate::synpass::MethodUnit;

/// Render a type to a whitespace-normalized string for table key comparison.
fn type_string(ty: &Type) -> String {
    normalize_type_string(&quote::quote!(#ty).to_string())
}

/// Collapse whitespace that `quote!` leaves around `*`, `<`, `>`, and `::`.
fn normalize_type_string(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            prev_space = true;
            continue;
        }
        if prev_space && !out.is_empty() {
            // Space preserved only between word characters, stripped adjacent to punctuation.
            let last = out.chars().next_back().unwrap();
            let word_boundary = (last.is_alphanumeric() || last == '_')
                && (ch.is_alphanumeric() || ch == '_');
            if word_boundary {
                out.push(' ');
            }
        }
        out.push(ch);
        prev_space = false;
    }
    out
}

fn is_cast_to(expr: &Expr, target: &str) -> bool {
    matches!(expr, Expr::Cast(cast) if type_string(&cast.ty) == target)
}

pub(crate) struct FileHandleCastPass;

impl VisitMut for FileHandleCastPass {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        visit_mut::visit_expr_mut(self, node);
        if let Expr::Cast(cast) = node {
            if type_string(&cast.ty) == "*mut FILE" {
                *cast.ty = parse_quote!(NativeFileHandle);
            }
        }
    }
}

pub(crate) struct NativeLayoutEngineCastPass;

impl NativeLayoutEngineCastPass {
    fn is_fontengine(expr: &Expr) -> bool {
        matches!(expr, Expr::Path(path) if path.path.is_ident("fontengine"))
    }

    fn is_stripped_cast_type(ty: &Type) -> bool {
        let rendered = type_string(ty);
        rendered == "FontHandle" || rendered == "*mut ()" || rendered.ends_with("DictionaryRef")
    }
}

impl VisitMut for NativeLayoutEngineCastPass {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        visit_mut::visit_expr_mut(self, node);
        if let Expr::Cast(cast) = node {
            if Self::is_fontengine(&cast.expr) && Self::is_stripped_cast_type(&cast.ty) {
                *node = (*cast.expr).clone();
            }
        }
    }
}

pub(crate) struct BoundaryReceiverPass<'a> {
    pub(crate) receiver_boundaries: &'a [&'a str],
}

impl BoundaryReceiverPass<'_> {
    fn self_call_segment(call: &ExprCall) -> Option<String> {
        let Expr::Path(path) = call.func.as_ref() else {
            return None;
        };
        let segments = &path.path.segments;
        if segments.len() != 2 || segments[0].ident != "Self" {
            return None;
        }
        Some(segments[1].ident.to_string())
    }

    fn already_has_receiver(call: &ExprCall) -> bool {
        matches!(
            call.args.first(),
            Some(Expr::Cast(cast))
                if type_string(&cast.ty) == "*mut PortableTexEngine<'_>"
        )
    }

    fn prepend_receiver(call: &mut ExprCall) {
        let receiver: Expr = parse_quote!(self as *mut PortableTexEngine<'_>);
        let mut new_args: Punctuated<Expr, Comma> = Punctuated::new();
        new_args.push(receiver);
        for arg in call.args.iter().cloned() {
            new_args.push(arg);
        }
        call.args = new_args;
    }
}

impl VisitMut for BoundaryReceiverPass<'_> {
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        visit_mut::visit_expr_call_mut(self, call);
        let Some(segment) = Self::self_call_segment(call) else {
            return;
        };
        let is_receiver_boundary = self.receiver_boundaries.contains(&segment.as_str());
        if !(is_receiver_boundary || segment == "abort_engine") {
            return;
        }
        if Self::already_has_receiver(call) {
            return;
        }
        // `boundary_prune_page_top` requires a second `false_0` argument that c2rust omits.
        if segment == "boundary_prune_page_top" && call.args.len() == 1 {
            let extra: Expr = parse_quote!(false_0);
            call.args.push(extra);
        }
        Self::prepend_receiver(call);
    }
}

pub(crate) struct HostServiceRenamePass<'a> {
    pub(crate) renames: &'a [(&'a str, &'a str)],
}

struct HostRename {
    from_segment: String,
    to_path: syn::Path,
    receiver: Option<Expr>,
}

impl HostServiceRenamePass<'_> {
    /// Parse rename table entries into [`HostRename`]s, extracting the callee path and optional receiver.
    fn parse_table(renames: &[(&str, &str)]) -> Vec<HostRename> {
        renames
            .iter()
            .map(|(raw, portable)| {
                let from_segment = raw
                    .trim_end_matches('(')
                    .trim_start_matches("Self::")
                    .to_string();
                let portable_open = portable
                    .find('(')
                    .expect("host rename portable form must contain `(`");
                let to_path: syn::Path = parse_str(&portable[..portable_open])
                    .expect("host rename portable callee must parse as a path");
                let arg_text = portable[portable_open + 1..].trim();
                let arg_text = arg_text.trim_end_matches(',').trim();
                let receiver = if arg_text.is_empty() {
                    None
                } else {
                    Some(parse_str(arg_text).expect("host rename receiver must parse as an expr"))
                };
                HostRename {
                    from_segment,
                    to_path,
                    receiver,
                }
            })
            .collect()
    }
}

struct HostRenameVisitor {
    renames: Vec<HostRename>,
}

impl VisitMut for HostRenameVisitor {
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        visit_mut::visit_expr_call_mut(self, call);
        let Expr::Path(path) = call.func.as_ref() else {
            return;
        };
        let segments = &path.path.segments;
        if segments.len() != 2 || segments[0].ident != "Self" {
            return;
        }
        let callee = segments[1].ident.to_string();
        let Some(rename) = self
            .renames
            .iter()
            .find(|rename| rename.from_segment == callee)
        else {
            return;
        };
        if let Expr::Path(path) = call.func.as_mut() {
            path.path = rename.to_path.clone();
        }
        // Idempotent: after rename the callee no longer matches any entry, so a re-run is a no-op.
        if let Some(receiver) = &rename.receiver {
            let mut new_args: Punctuated<Expr, Comma> = Punctuated::new();
            new_args.push(receiver.clone());
            for arg in call.args.iter().cloned() {
                new_args.push(arg);
            }
            call.args = new_args;
        }
    }
}

impl VisitMut for HostServiceRenamePass<'_> {
    fn visit_impl_item_fn_mut(&mut self, node: &mut syn::ImplItemFn) {
        let mut visitor = HostRenameVisitor {
            renames: Self::parse_table(self.renames),
        };
        visitor.visit_impl_item_fn_mut(node);
    }
}

pub(crate) struct VectorPointerPass<'a> {
    pub(crate) vector_fields: &'a BTreeSet<String>,
}

impl VectorPointerPass<'_> {
    fn state_vector_field(&self, expr: &Expr) -> Option<String> {
        let Expr::Field(field) = expr else {
            return None;
        };
        let member = match &field.member {
            syn::Member::Named(ident) => ident.to_string(),
            syn::Member::Unnamed(_) => return None,
        };
        let Expr::Field(inner) = field.base.as_ref() else {
            return None;
        };
        if !matches!(&inner.member, syn::Member::Named(ident) if ident == "state") {
            return None;
        }
        if !matches!(inner.base.as_ref(), Expr::Path(path) if path.path.is_ident("self")) {
            return None;
        }
        self.vector_fields.contains(&member).then_some(member)
    }

    fn materialize(&self, expr: &mut Expr) {
        if self.state_vector_field(expr).is_some() {
            let field = expr.clone();
            *expr = parse_quote!(#field.as_mut_ptr());
        }
    }
}

impl VisitMut for VectorPointerPass<'_> {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        visit_mut::visit_expr_mut(self, node);
        if let Expr::MethodCall(call) = node {
            if call.method == "is_null"
                && call.args.is_empty()
                && self.state_vector_field(&call.receiver).is_some()
            {
                call.method = parse_quote!(is_empty);
            }
        }
    }

    fn visit_local_mut(&mut self, local: &mut Local) {
        visit_mut::visit_local_mut(self, local);
        if let Some(init) = &mut local.init {
            if init.diverge.is_none() {
                self.materialize(&mut init.expr);
            }
        }
    }

    fn visit_expr_assign_mut(&mut self, assign: &mut syn::ExprAssign) {
        visit_mut::visit_expr_assign_mut(self, assign);
        self.materialize(&mut assign.right);
    }
}

pub(crate) struct WidenedArrayCastPass;

impl WidenedArrayCastPass {
    fn widen_rhs(rhs: &mut Expr) {
        match rhs {
            Expr::Cast(cast) if type_string(&cast.ty) == "i16" => {
                *cast.ty = parse_quote!(integer);
            }
            Expr::Field(field)
                if matches!(&field.member, syn::Member::Named(ident) if ident == "B1") =>
            {
                let inner = rhs.clone();
                *rhs = parse_quote!(#inner as integer);
            }
            _ => {}
        }
    }
}

impl VisitMut for WidenedArrayCastPass {
    fn visit_expr_assign_mut(&mut self, assign: &mut syn::ExprAssign) {
        visit_mut::visit_expr_assign_mut(self, assign);
        if is_widened_array_index(&assign.left) {
            Self::widen_rhs(&mut assign.right);
        }
    }
}

/// Cast `.u.B{0..3}` through `u16`/`i32`, exclude the `.hh.u.B*` `two_halves` view.
pub(crate) struct FourQuarterCastPass;

impl FourQuarterCastPass {
    /// True for `.u.B{0..3}` field accesses where the parent is not `.hh` (which would be `two_halves`).
    fn is_four_quarter_field(expr: &Expr) -> bool {
        let Expr::Field(field) = expr else {
            return false;
        };
        let syn::Member::Named(b) = &field.member else {
            return false;
        };
        if !matches!(b.to_string().as_str(), "B0" | "B1" | "B2" | "B3") {
            return false;
        }
        let Expr::Field(u_field) = field.base.as_ref() else {
            return false;
        };
        if !matches!(&u_field.member, syn::Member::Named(m) if m == "u") {
            return false;
        }
        // `.hh.u.B*` is the `two_halves` i16 view, exclude it.
        if let Expr::Field(parent) = u_field.base.as_ref() {
            if matches!(&parent.member, syn::Member::Named(m) if m == "hh") {
                return false;
            }
        }
        true
    }

    fn already_cast_to(expr: &Expr, ty: &str) -> bool {
        matches!(expr, Expr::Cast(cast) if type_string(&cast.ty) == ty)
    }
}

impl VisitMut for FourQuarterCastPass {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Recurse first into the value expression, the assignment target must remain a place expression.
        if let Expr::Assign(assign) = expr {
            if Self::is_four_quarter_field(&assign.left) {
                self.visit_expr_mut(&mut assign.right);
                if !Self::already_cast_to(&assign.right, "u16") {
                    let rhs = assign.right.clone();
                    *assign.right = parse_quote!((#rhs) as u16);
                }
                return;
            }
        }
        visit_mut::visit_expr_mut(self, expr);
        // Reads that are not assignment targets are widened back to i32 (u16 zero-extends, harmless).
        if Self::is_four_quarter_field(expr) && !Self::already_cast_to(expr, "i32") {
            let inner = expr.clone();
            *expr = parse_quote!((#inner) as i32);
        }
    }
}

pub(crate) struct OtFontGetPass;

impl OtFontGetPass {
    fn is_ot_font_get(call: &ExprCall) -> bool {
        let Expr::Path(path) = call.func.as_ref() else {
            return false;
        };
        path.path.segments.len() == 1
            && matches!(
                path.path.segments[0].ident.to_string().as_str(),
                "otfontget" | "otfontget1" | "otfontget2" | "otfontget3"
            )
    }

    /// Extract `n` from the c2rust `fontlayoutengine.offset(n as isize)` argument.
    fn font_index(arg: &Expr) -> Option<Expr> {
        let Expr::Cast(cast) = arg else {
            return None;
        };
        let Expr::Unary(unary) = cast.expr.as_ref() else {
            return None;
        };
        let Expr::MethodCall(method_call) = unary.expr.as_ref() else {
            return None;
        };
        if method_call.method != "offset" {
            return None;
        }
        let index = method_call.args.first()?;
        match index {
            Expr::Cast(index_cast) => Some((*index_cast.expr).clone()),
            other => Some(other.clone()),
        }
    }
}

impl VisitMut for OtFontGetPass {
    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        visit_mut::visit_expr_call_mut(self, call);
        if !Self::is_ot_font_get(call) {
            return;
        }
        let args: Vec<Expr> = call.args.iter().cloned().collect();
        let (Some(what), Some(engine_arg)) = (args.first(), args.get(1)) else {
            return;
        };
        let Some(index) = Self::font_index(engine_arg) else {
            return;
        };
        let zero: Expr = parse_quote!(0);
        let param1 = args.get(2).cloned().unwrap_or_else(|| zero.clone());
        let param2 = args.get(3).cloned().unwrap_or_else(|| zero.clone());
        let param3 = args.get(4).cloned().unwrap_or_else(|| zero.clone());
        let what = what.clone();

        call.func = Box::new(parse_quote!(Self::ot_font_get));
        let mut new_args: Punctuated<Expr, Comma> = Punctuated::new();
        new_args.push(parse_quote!(self as *mut PortableTexEngine<'_>));
        new_args.push(what);
        new_args.push(index);
        new_args.push(param1);
        new_args.push(param2);
        new_args.push(param3);
        call.args = new_args;
    }
}

pub(crate) struct KnownTypeMismatchPass;

struct ArgCast {
    arg_form: &'static str,
    cast_to: &'static str,
}

/// Render an expression to a whitespace-normalized string for `ArgCast::arg_form` comparison.
fn expr_form(expr: &Expr) -> String {
    normalize_type_string(&quote::quote!(#expr).to_string())
}

impl KnownTypeMismatchPass {
    /// Idempotent: arguments already cast to the target type are left alone.
    fn apply_arg_casts(args: &mut Punctuated<Expr, Comma>, casts: &[ArgCast]) {
        for cast in casts {
            for arg in args.iter_mut() {
                if is_cast_to(arg, cast.cast_to) {
                    continue;
                }
                if expr_form(arg) == cast.arg_form {
                    let inner = arg.clone();
                    let target: Type =
                        parse_str(cast.cast_to).expect("type-mismatch cast target must parse");
                    *arg = parse_quote!(#inner as #target);
                }
            }
        }
    }
}

impl VisitMut for KnownTypeMismatchPass {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        visit_mut::visit_expr_mut(self, node);
        match node {
            Expr::MethodCall(call) => {
                let method = call.method.to_string();
                if let Some(casts) = method_call_casts(&method) {
                    Self::apply_arg_casts(&mut call.args, casts);
                }
            }
            Expr::Assign(assign) => {
                retype_assignment_rhs(assign);
            }
            _ => {}
        }
    }
}

fn method_call_casts(method: &str) -> Option<&'static [ArgCast]> {
    match method {
        "zreconstitute" => Some(&[
            ArgCast { arg_form: "j", cast_to: "smallnumber" },
            ArgCast { arg_form: "self.state.hn", cast_to: "smallnumber" },
            ArgCast { arg_form: "l_0", cast_to: "smallnumber" },
            ArgCast { arg_form: "i", cast_to: "smallnumber" },
            ArgCast {
                arg_form: "*self.state.fontbchar.offset(self.state.hf as isize)",
                cast_to: "halfword",
            },
        ]),
        "zprintchar" => Some(&[
            ArgCast {
                arg_form: "*self.state.strpool.offset(j as isize)",
                cast_to: "ASCIIcode",
            },
            ArgCast {
                arg_form: "*self.state.strpool.offset(k as isize)",
                cast_to: "ASCIIcode",
            },
        ]),
        "zmorename" => Some(&[ArgCast {
            arg_form: "*self.state.strpool.offset(i as isize)",
            cast_to: "ASCIIcode",
        }]),
        "znewcharacter" => Some(&[
            ArgCast { arg_form: "self.state.curval", cast_to: "UTF16code" },
            ArgCast { arg_form: "self.state.curchr", cast_to: "UTF16code" },
            ArgCast { arg_form: "self.state.curc", cast_to: "UTF16code" },
        ]),
        "zeffectivechar" => Some(&[ArgCast { arg_form: "c_0", cast_to: "quarterword" }]),
        _ => None,
    }
}

/// Fix `fontbc`/`fontec` stores to `UTF16code` and `hu`/`hc` reads assigned to `.hh.u.B1` to `i16`.
fn retype_assignment_rhs(assign: &mut syn::ExprAssign) {
    if assign_target_ends_with_field(&assign.left, "B1")
        && is_widened_array_index(&assign.right)
        && !is_cast_to(&assign.right, "i16")
    {
        let inner = (*assign.right).clone();
        *assign.right = parse_quote!(#inner as i16);
        return;
    }

    let field = match assign_target_field(&assign.left) {
        Some(field) => field,
        None => return,
    };
    let (from, to) = match field.as_str() {
        "fontbc" | "fontec" => ("eightbits", "UTF16code"),
        _ => return,
    };
    if let Expr::Cast(cast) = assign.right.as_mut() {
        if type_string(&cast.ty) == from {
            *cast.ty = parse_str::<Type>(to).expect("retype target must parse");
        }
    }
}

fn is_widened_array_index(expr: &Expr) -> bool {
    let Expr::Index(index) = expr else {
        return false;
    };
    let Expr::Field(field) = index.expr.as_ref() else {
        return false;
    };
    if !matches!(&field.member, syn::Member::Named(ident) if ident == "hu" || ident == "hc") {
        return false;
    }
    matches!(field.base.as_ref(), Expr::Field(inner)
        if matches!(&inner.member, syn::Member::Named(ident) if ident == "state"))
}

fn assign_target_ends_with_field(lhs: &Expr, name: &str) -> bool {
    matches!(lhs, Expr::Field(field)
        if matches!(&field.member, syn::Member::Named(ident) if ident == name))
}

/// Extract the field ident from an assignment target of the form `*self.state.<field>.offset(..)`.
fn assign_target_field(lhs: &Expr) -> Option<String> {
    let Expr::Unary(unary) = lhs else {
        return None;
    };
    if !matches!(unary.op, syn::UnOp::Deref(_)) {
        return None;
    }
    let Expr::MethodCall(call) = unary.expr.as_ref() else {
        return None;
    };
    if call.method != "offset" {
        return None;
    }
    let Expr::Field(field) = call.receiver.as_ref() else {
        return None;
    };
    let member = match &field.member {
        syn::Member::Named(ident) => ident.to_string(),
        syn::Member::Unnamed(_) => return None,
    };
    matches!(field.base.as_ref(), Expr::Field(inner)
        if matches!(&inner.member, syn::Member::Named(ident) if ident == "state"))
        .then_some(member)
}

/// Run all structural AST passes over one method block, pass order mirrors `RawFunction::patch`.
pub(crate) fn run_all(
    block: &str,
    receiver_boundaries: &[&str],
    host_renames: &[(&str, &str)],
    vector_fields: &BTreeSet<String>,
) -> String {
    let mut unit = MethodUnit::parse(block);

    let changed = {
        let method = unit.method_mut();

        // Snapshot before any pass so an untouched function is returned byte-identical.
        let before = quote::quote!(#method).to_string();

        FileHandleCastPass.visit_impl_item_fn_mut(method);
        NativeLayoutEngineCastPass.visit_impl_item_fn_mut(method);
        BoundaryReceiverPass {
            receiver_boundaries,
        }
        .visit_impl_item_fn_mut(method);
        HostServiceRenamePass {
            renames: host_renames,
        }
        .visit_impl_item_fn_mut(method);
        VectorPointerPass { vector_fields }.visit_impl_item_fn_mut(method);
        KnownTypeMismatchPass.visit_impl_item_fn_mut(method);
        WidenedArrayCastPass.visit_impl_item_fn_mut(method);
        OtFontGetPass.visit_impl_item_fn_mut(method);
        FourQuarterCastPass.visit_impl_item_fn_mut(method);

        let after = quote::quote!(#method).to_string();
        before != after
    };

    if !changed {
        return block.to_string();
    }
    unit.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Uses the real production tables, not stubs.
    fn boundaries() -> &'static [&'static str] {
        crate::boundary::RECEIVER_BOUNDARY_CALLS
    }
    fn host_renames() -> &'static [(&'static str, &'static str)] {
        crate::boundary::HOST_SERVICE_CALL_RENAMES
    }

    fn run(body: &str, vectors: &[&str]) -> String {
        let vector_fields: BTreeSet<String> =
            vectors.iter().map(|name| (*name).to_string()).collect();
        run_all(
            &format!("pub(crate) unsafe fn f(&mut self) {{\n{body}\n}}"),
            boundaries(),
            host_renames(),
            &vector_fields,
        )
    }

    #[test]
    fn p1_file_handle_cast_is_rewritten() {
        let out = run("let x = p as *mut FILE;", &[]);
        assert!(out.contains("as NativeFileHandle"), "{out}");
        assert!(!out.contains("as *mut FILE"), "{out}");
    }

    #[test]
    fn p2_fontengine_cast_is_stripped() {
        let out = run("let x = g(fontengine as FontHandle);", &[]);
        assert!(out.contains("g(fontengine)"), "{out}");
        assert!(!out.contains("fontengine as FontHandle"), "{out}");
    }

    #[test]
    fn p4_boundary_receiver_is_prepended_without_dangling_comma() {
        let out = run("Self::boundary_build_page();", &[]);
        assert!(
            out.contains("Self::boundary_build_page(self as *mut PortableTexEngine<'_>)"),
            "{out}"
        );
        // No dangling `, )` on the zero-argument boundary.
        assert!(!out.contains("<'_>,)") && !out.contains("<'_>, )"), "{out}");
    }

    #[test]
    fn p4_abort_engine_gets_receiver() {
        let out = run("Self::abort_engine(0 as integer);", &[]);
        assert!(
            out.contains("Self::abort_engine(self as *mut PortableTexEngine<'_>, 0 as integer)"),
            "{out}"
        );
    }

    #[test]
    fn p4_is_idempotent_on_existing_receiver() {
        let out = run(
            "Self::boundary_build_page(self as *mut PortableTexEngine<'_>);",
            &[],
        );
        assert_eq!(
            out.matches("self as *mut PortableTexEngine<'_>").count(),
            1,
            "{out}"
        );
    }

    #[test]
    fn p5_host_service_is_renamed_and_receiver_injected() {
        let out = run("let x = Self::host_usingOpenType(f);", &[]);
        assert!(
            out.contains("Self::using_opentype(self as *mut PortableTexEngine<'_>, f)"),
            "{out}"
        );
        assert!(!out.contains("host_usingOpenType"), "{out}");
    }

    #[test]
    fn p5_no_receiver_variant_only_renames() {
        // `ot_part_count` maps to a no-receiver portable call.
        let out = run("let x = Self::host_ot_part_count(a, b);", &[]);
        assert!(out.contains("Self::opentype_part_count(a, b)"), "{out}");
        assert!(!out.contains("PortableTexEngine"), "{out}");
    }

    #[test]
    fn p6_zreconstitute_args_are_cast() {
        let out = run(
            "let j = (&mut *(self as *mut PortableTexEngine<'_>)).zreconstitute(j, self.state.hn, bchar_0, hyfchar);",
            &[],
        );
        assert!(out.contains("j as smallnumber"), "{out}");
        assert!(out.contains("self.state.hn as smallnumber"), "{out}");
        // `bchar_0`/`hyfchar` are not in the descriptor and stay bare.
        assert!(out.contains("bchar_0,") || out.contains("bchar_0\n"), "{out}");
    }

    #[test]
    fn p6_is_idempotent_on_already_cast_arg() {
        let out = run(
            "let j = (&mut *(self as *mut PortableTexEngine<'_>)).zreconstitute(j as smallnumber, self.state.hn as smallnumber, bchar_0, hyfchar);",
            &[],
        );
        assert!(!out.contains("as smallnumber as smallnumber"), "{out}");
    }

    #[test]
    fn p6_fontbc_assignment_is_retyped() {
        let out = run(
            "*self.state.fontbc.offset(f_0 as isize) = bc as eightbits;",
            &[],
        );
        assert!(out.contains("bc as UTF16code"), "{out}");
        assert!(!out.contains("bc as eightbits"), "{out}");
    }

    #[test]
    fn p7_vector_pointer_is_materialized() {
        let out = run("let mut mem: *mut memoryword = self.state.zmem;", &["zmem"]);
        assert!(out.contains("self.state.zmem.as_mut_ptr()"), "{out}");
    }

    #[test]
    fn p7_is_null_becomes_is_empty() {
        let out = run("if self.state.zmem.is_null() { return; }", &["zmem"]);
        assert!(out.contains("self.state.zmem.is_empty()"), "{out}");
        assert!(!out.contains("self.state.zmem.is_null()"), "{out}");
    }

    #[test]
    fn p7_non_vector_field_is_untouched() {
        let out = run("let mut x = self.state.curval;", &[]);
        assert!(!out.contains("as_mut_ptr"), "{out}");
        assert!(out.contains("self.state.curval"), "{out}");
    }

    #[test]
    fn p8_widened_array_b1_read_is_cast() {
        let out = run(
            "self.state.hu[0 as usize] = (*mem.offset(p as isize)).hh.u.B1;",
            &[],
        );
        assert!(out.contains(".B1 as integer"), "{out}");
    }

    #[test]
    fn p8_widened_array_i16_cast_becomes_integer() {
        let out = run("self.state.hc[i as usize] = c as i16;", &[]);
        assert!(out.contains("c as integer"), "{out}");
        assert!(!out.contains("c as i16"), "{out}");
    }

    #[test]
    fn unchanged_body_is_returned_byte_identical() {
        let block = "pub(crate) unsafe fn f(&mut self) {\n    let x = 1 as i32;\n    self.harmless();\n}";
        let out = run_all(block, boundaries(), host_renames(), &BTreeSet::new());
        assert_eq!(out, block, "an untouched body must pass through verbatim");
    }

    #[test]
    fn run_all_is_idempotent() {
        let block = "pub(crate) unsafe fn f(&mut self) {\n    Self::boundary_build_page();\n    let x = p as *mut FILE;\n}";
        let once = run_all(block, boundaries(), host_renames(), &BTreeSet::new());
        let twice = run_all(&once, boundaries(), host_renames(), &BTreeSet::new());
        assert_eq!(once, twice, "second pass must be a no-op");
    }
}
