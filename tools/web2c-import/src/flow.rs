//! Rewrites abort reachable functions to return `EngineFlow<T>` and thread `?` through terminal calls.

use std::collections::BTreeSet;

use syn::visit_mut::{self, VisitMut};
use syn::{parse_quote, parse_str, Expr, ImplItemFn, ItemImpl, ReturnType, Stmt};

use crate::synpass::{indent_method, render_method, single_method_mut, single_method_ref};

/// Rewrite one `pub(crate) unsafe fn` block string into its `EngineFlow` form.
pub(crate) fn rewrite_fallible_function(block: &str, triggers: &FlowTriggers) -> String {
    let wrapped = format!("impl __FlowWrap {{\n{block}\n}}\n");
    let mut item_impl: ItemImpl = parse_str(&wrapped)
        .unwrap_or_else(|error| panic!("flow: failed to parse wrapped function: {error}\n{block}"));

    let method = single_method_mut(&mut item_impl);
    let mut rewriter = AbortRewriter {
        triggers,
        skip_wrap: false,
    };
    rewriter.rewrite_method(method);

    let method = single_method_ref(&item_impl);
    let rendered = render_method(method);
    indent_method(&rendered)
}

/// Rewrite each named bridge function in `source` to its `EngineFlow` form.
pub(crate) fn rewrite_named_functions(
    mut source: String,
    names: &[&str],
    triggers: &FlowTriggers,
) -> String {
    for name in names {
        let (start, end) = locate_function_block(&source, name)
            .unwrap_or_else(|| panic!("flow: bridge function `{name}` not found in runtime source"));
        let block = source[start..end].to_string();
        // `rewrite_fallible_function` emits with the extracted block's indentation.
        let rewritten = rewrite_fallible_function(block.trim(), triggers);
        source.replace_range(start..end, &rewritten);
    }
    source
}

fn locate_function_block(source: &str, name: &str) -> Option<(usize, usize)> {
    // Match at impl member indentation to avoid catching call sites or nested references.
    let needle = format!(" fn {name}(");
    let sig_pos = source.find(&needle)?;
    // Start at the signature line so the rendered method preserves member indentation.
    let start = source[..sig_pos].rfind('\n').map(|nl| nl + 1)?;
    let brace_open = source[sig_pos..].find('{')? + sig_pos;
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut idx = brace_open;
    while idx < bytes.len() {
        match bytes[idx] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, idx + 1));
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

/// The call identifier classes that the rewriter wraps.
pub(crate) struct FlowTriggers {
    fallible: BTreeSet<String>,
    diverging: BTreeSet<String>,
}

impl FlowTriggers {
    fn classify(&self, name: &str) -> Option<TriggerKind> {
        if self.diverging.contains(name) {
            Some(TriggerKind::Diverging)
        } else if self.fallible.contains(name) {
            Some(TriggerKind::Fallible)
        } else {
            None
        }
    }

    /// Register bridge functions as fallible triggers.
    pub(crate) fn register_prelude_bridges(&mut self) {
        for name in PRELUDE_BRIDGE_FUNCTIONS
            .iter()
            .chain(HAND_EDITED_BRIDGE_FUNCTIONS)
        {
            self.fallible.insert((*name).to_string());
        }
    }
}

/// Hand written prelude functions that propagate aborts and are rewritten by the `syn` pass.
pub(crate) const PRELUDE_BRIDGE_FUNCTIONS: &[&str] = &[
    "load_pool_strings",
    "boundary_open_log_file",
    "boundary_special_out",
    "boundary_write_whatsit",
    "begin_primary_input_raw",
    "znotaatfonterror",
    "znotaatgrfonterror",
    "stack_glyph_into_box",
    "stack_glue_into_box",
    "zbuildopentypeassembly",
];

/// Bridge functions excluded from automatic rewrite but registered as fallible triggers.
const HAND_EDITED_BRIDGE_FUNCTIONS: &[&str] = &[
    "intern_static_pool_string",
    "surface_error",
    "sandbox_reject",
    "sandbox_open_math",
    "sandbox_close_math",
    "sandbox_tick",
];

#[derive(Clone, Copy)]
enum TriggerKind {
    Fallible,
    Diverging,
}

/// Build the set of call identifiers that trigger a `?` wrap.
pub(crate) fn build_trigger_set(
    fallible: &BTreeSet<String>,
    boundary_renames: &[(&str, &str)],
) -> FlowTriggers {
    // c2rust `-> !` functions plus terminals that abort.
    let diverging_names: BTreeSet<String> = [
        "zfatalerror",
        "zoverflow",
        "zconfusion",
        "boundary_jump_out",
        "abort_engine",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect();

    let mut fallible_set = BTreeSet::new();
    for name in fallible {
        match boundary_renames.iter().find(|(input, _)| input == name) {
            // Only `boundary_build_page` and `boundary_jump_out` return `EngineFlow`.
            Some((_, output)) => {
                let output = (*output).to_string();
                if output == "boundary_build_page" {
                    fallible_set.insert(output);
                }
                // `boundary_jump_out` is in the diverging set.
            }
            None => {
                if !diverging_names.contains(name) {
                    fallible_set.insert(name.clone());
                }
            }
        }
    }
    FlowTriggers {
        fallible: fallible_set,
        diverging: diverging_names,
    }
}

struct AbortRewriter<'a> {
    triggers: &'a FlowTriggers,
    /// Set when visiting the direct operand of an existing `?` (`Expr::Try`).
    skip_wrap: bool,
}

impl AbortRewriter<'_> {
    fn rewrite_method(&mut self, method: &mut ImplItemFn) {
        // True for diverging sources (`-> !`) whose body ends via a terminal.
        let diverging = matches!(method.sig.output, ReturnType::Type(_, ref ty) if matches!(**ty, syn::Type::Never(_)));

        rewrite_signature(&mut method.sig.output);

        visit_mut::visit_block_mut(self, &mut method.block);

        if !diverging {
            ok_wrap_tail(&mut method.block);
        }
    }
}

impl VisitMut for AbortRewriter<'_> {
    fn visit_expr_mut(&mut self, node: &mut Expr) {
        // Existing `?` recurses into the inner expression with `skip_wrap` set.
        if let Expr::Try(try_expr) = node {
            let previous = self.skip_wrap;
            self.skip_wrap = true;
            self.visit_expr_mut(&mut try_expr.expr);
            self.skip_wrap = previous;
            return;
        }

        let suppress = self.skip_wrap;
        self.skip_wrap = false;

        // Recurse into children first so nested fallible calls are wrapped before this node.
        visit_mut::visit_expr_mut(self, node);

        if suppress {
            return;
        }

        match self.trigger_kind(node) {
            Some(TriggerKind::Fallible) => {
                let inner = node.clone();
                *node = parse_quote!(#inner?);
            }
            Some(TriggerKind::Diverging) => {
                // `match <call>? {}` preserves `-> !` semantics.
                let inner = node.clone();
                *node = parse_quote!(match #inner? {});
            }
            None => {}
        }
    }

    fn visit_expr_return_mut(&mut self, node: &mut syn::ExprReturn) {
        visit_mut::visit_expr_return_mut(self, node);
        match node.expr.take() {
            Some(expr) => node.expr = Some(Box::new(parse_quote!(Ok(#expr)))),
            None => node.expr = Some(Box::new(parse_quote!(Ok(())))),
        }
    }
}

impl AbortRewriter<'_> {
    /// Classifies a call expression by matching the callee name.
    fn trigger_kind(&self, node: &Expr) -> Option<TriggerKind> {
        let callee = match node {
            Expr::MethodCall(call) => call.method.to_string(),
            Expr::Call(call) => {
                let Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                path.path.segments.last()?.ident.to_string()
            }
            _ => return None,
        };
        self.triggers.classify(&callee)
    }
}

/// Rewrites a return type to its `EngineFlow` form.
fn rewrite_signature(output: &mut ReturnType) {
    let new_output: ReturnType = match output {
        ReturnType::Default => parse_quote!(-> EngineFlow<()>),
        ReturnType::Type(_, ty) => {
            if matches!(**ty, syn::Type::Never(_)) {
                parse_quote!(-> EngineFlow<core::convert::Infallible>)
            } else {
                let inner = ty.clone();
                parse_quote!(-> EngineFlow<#inner>)
            }
        }
    };
    *output = new_output;
}

/// Wraps the tail of `block` in `Ok(...)` or appends `Ok(())`.
fn ok_wrap_tail(block: &mut syn::Block) {
    match block.stmts.last_mut() {
        // Already type `!` (coerces to `EngineFlow<T>`), wrapping in `Ok(..)` would be unreachable.
        Some(Stmt::Expr(expr, _)) if expr_diverges(expr) => {}
        Some(Stmt::Expr(expr, None)) => {
            let inner = expr.clone();
            *expr = parse_quote!(Ok(#inner));
        }
        _ => {
            let ok: Expr = parse_quote!(Ok(()));
            block.stmts.push(Stmt::Expr(ok, None));
        }
    }
}

/// Returns true if `expr` conservatively always diverges.
fn expr_diverges(expr: &Expr) -> bool {
    match expr {
        Expr::Return(_) => true,
        Expr::Match(m) => {
            // Empty match diverges, otherwise every arm must diverge.
            !m.arms.is_empty() && m.arms.iter().all(|arm| expr_diverges(&arm.body))
                || m.arms.is_empty()
        }
        Expr::If(if_expr) => match &if_expr.else_branch {
            Some((_, else_expr)) => {
                block_diverges(&if_expr.then_branch) && expr_diverges(else_expr)
            }
            None => false,
        },
        Expr::Block(b) => block_diverges(&b.block),
        Expr::Loop(loop_expr) => !block_contains_break(&loop_expr.body),
        _ => false,
    }
}

fn block_diverges(block: &syn::Block) -> bool {
    match block.stmts.last() {
        Some(Stmt::Expr(expr, _)) => expr_diverges(expr),
        _ => false,
    }
}

/// Returns true if `block` contains a direct `break`.
fn block_contains_break(block: &syn::Block) -> bool {
    use syn::visit::Visit;
    struct BreakFinder {
        found: bool,
    }
    impl<'ast> Visit<'ast> for BreakFinder {
        fn visit_expr_break(&mut self, _: &'ast syn::ExprBreak) {
            self.found = true;
        }
        // Do not descend into nested loops: their breaks belong to them.
        fn visit_expr_loop(&mut self, _: &'ast syn::ExprLoop) {}
        fn visit_expr_while(&mut self, _: &'ast syn::ExprWhile) {}
        fn visit_expr_for_loop(&mut self, _: &'ast syn::ExprForLoop) {}
    }
    let mut finder = BreakFinder { found: false };
    finder.visit_block(block);
    finder.found
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Fallible: `error`, `getavail`. Diverging: `zfatalerror`, `boundary_jump_out`, `abort_engine`.
    fn triggers() -> FlowTriggers {
        let fallible = ["error", "getavail"]
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        let diverging = ["zfatalerror", "boundary_jump_out", "abort_engine"]
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        FlowTriggers {
            fallible,
            diverging,
        }
    }

    #[test]
    fn unit_return_becomes_engineflow_unit_and_ok_wraps_tail() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) {\n    let x = 1;\n}",
            &triggers(),
        );
        assert!(out.contains("-> EngineFlow<()>"), "{out}");
        assert!(out.contains("Ok(())"), "{out}");
    }

    #[test]
    fn fallible_call_is_question_wrapped() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) {\n    self.error();\n}",
            &triggers(),
        );
        assert!(out.contains("self.error()?"), "{out}");
    }

    #[test]
    fn non_trigger_call_is_not_wrapped() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) {\n    self.harmless();\n}",
            &triggers(),
        );
        assert!(!out.contains("harmless()?"), "{out}");
    }

    #[test]
    fn value_return_is_ok_wrapped_and_typed() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) -> i32 {\n    return 7;\n}",
            &triggers(),
        );
        assert!(out.contains("-> EngineFlow<i32>"), "{out}");
        assert!(out.contains("return Ok(7)"), "{out}");
    }

    #[test]
    fn diverging_source_becomes_infallible_and_match_terminal() {
        // A `-> !` source whose body ends with a diverging terminal.
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) -> ! {\n    Self::boundary_jump_out(self);\n}",
            &triggers(),
        );
        assert!(
            out.contains("-> EngineFlow<core::convert::Infallible>"),
            "{out}"
        );
        assert!(out.contains("match Self::boundary_jump_out(self)? {}"), "{out}");
    }

    #[test]
    fn diverging_call_in_statement_position_uses_match() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) {\n    self.zfatalerror(1);\n    let _ = 2;\n}",
            &triggers(),
        );
        assert!(out.contains("match self.zfatalerror(1)? {}"), "{out}");
    }

    #[test]
    fn already_wrapped_call_is_not_double_wrapped() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) {\n    self.error()?;\n}",
            &triggers(),
        );
        assert!(out.contains("self.error()?"), "{out}");
        assert!(!out.contains("self.error()??"), "{out}");
    }

    #[test]
    fn diverging_tail_match_is_not_ok_wrapped() {
        let out = rewrite_fallible_function(
            "pub(crate) unsafe fn f(&mut self) -> i32 {\n    match self.k {\n        0 => return 1,\n        _ => self.zfatalerror(2),\n    }\n}",
            &triggers(),
        );
        // Both arms diverge, so no trailing Ok(()) is appended.
        let ok_unit_count = out.matches("Ok(())").count();
        assert_eq!(ok_unit_count, 0, "unexpected Ok(()) tail in {out}");
    }

    #[test]
    fn build_trigger_set_drops_non_fatal_boundaries() {
        // Boundary routes split into fallible, diverging, and plain methods.
        let fallible = ["zshipout", "jumpout", "buildpage", "error"]
            .iter()
            .map(|name| (*name).to_string())
            .collect();
        let renames = &[
            ("zshipout", "boundary_shipout"),
            ("jumpout", "boundary_jump_out"),
            ("buildpage", "boundary_build_page"),
        ][..];
        let triggers = build_trigger_set(&fallible, renames);
        assert!(triggers.fallible.contains("boundary_build_page"));
        assert!(triggers.fallible.contains("error"));
        assert!(!triggers.fallible.contains("boundary_shipout"));
        assert!(triggers.diverging.contains("boundary_jump_out"));
    }
}
