//! Shared `syn` parse, render, and splice harness for method AST passes.

use proc_macro2::TokenStream;
use syn::{parse_str, ImplItem, ImplItemFn, ItemImpl};

/// One method wrapped in a throwaway `impl __Wrap { .. }` for parsing and rendering.
pub(crate) struct MethodUnit {
    item_impl: ItemImpl,
}

impl MethodUnit {
    /// Parses one impl member method block.
    pub(crate) fn parse(block: &str) -> Self {
        let wrapped = format!("impl __Wrap {{\n{block}\n}}\n");
        let item_impl: ItemImpl = parse_str(&wrapped).unwrap_or_else(|error| {
            panic!("synpass: failed to parse wrapped function: {error}\n{block}")
        });
        Self { item_impl }
    }

    pub(crate) fn method_mut(&mut self) -> &mut ImplItemFn {
        single_method_mut(&mut self.item_impl)
    }

    /// Renders the method back to source text ready to splice verbatim.
    pub(crate) fn render(&self) -> String {
        let method = single_method_ref(&self.item_impl);
        let rendered = render_method(method);
        indent_method(&rendered)
    }
}

pub(crate) fn single_method_mut(item_impl: &mut ItemImpl) -> &mut ImplItemFn {
    for item in &mut item_impl.items {
        if let ImplItem::Fn(method) = item {
            return method;
        }
    }
    panic!("synpass: wrapped impl had no method");
}

pub(crate) fn single_method_ref(item_impl: &ItemImpl) -> &ImplItemFn {
    for item in &item_impl.items {
        if let ImplItem::Fn(method) = item {
            return method;
        }
    }
    panic!("synpass: wrapped impl had no method");
}

/// Renders a method through `prettyplease` by wrapping it in a throwaway impl.
pub(crate) fn render_method(method: &ImplItemFn) -> String {
    let tokens: TokenStream = quote::quote! {
        impl __Wrap {
            #method
        }
    };
    let file: syn::File = syn::parse2(tokens).expect("synpass: re-parse for render failed");
    let unparsed = prettyplease::unparse(&file);
    debug_assert!(
        parse_str::<ItemImpl>(&unparsed).is_ok(),
        "synpass: rendered method did not re-parse as an impl:\n{unparsed}"
    );
    extract_impl_body(&unparsed)
}

/// Strips the `impl __Wrap {` wrapper from a prettyplease rendered impl.
pub(crate) fn extract_impl_body(unparsed: &str) -> String {
    let open = unparsed
        .find('{')
        .expect("synpass: rendered impl missing opening brace");
    let close = unparsed
        .rfind('}')
        .expect("synpass: rendered impl missing closing brace");
    unparsed[open + 1..close].trim_matches('\n').to_string()
}

/// Trims leading and trailing blank lines from a prettyplease rendered method.
pub(crate) fn indent_method(rendered: &str) -> String {
    rendered.trim_matches('\n').to_string()
}
