//! Internal proc-macro helpers for `step-io` entity handlers.
//!
//! Provides `#[step_entity]` / `#[step_entity_complex]` attribute macros
//! that emit the const NAME / PASS_LEVEL members + `ENTITY_HANDLERS`
//! registry entry that every handler module would otherwise hand-roll.
//!
//! Path policy: macro output uses `crate::...` relative paths so it
//! resolves against the caller crate (`step-io`). step-io-macros must not
//! reference step-io directly (would be a circular dependency).

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Expr, ExprArray, ItemImpl, LitStr, Path, Token, Type, parse::Parse, parse::ParseStream,
    parse_macro_input,
};

/// `name = "ENTITY_NAME", pass = PassLevel::Variant`
struct SimpleArgs {
    name: LitStr,
    pass: Path,
}

impl Parse for SimpleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut pass: Option<Path> = None;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match ident.to_string().as_str() {
                "name" => name = Some(input.parse()?),
                "pass" => pass = Some(input.parse()?),
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown step_entity argument: {other}"),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(SimpleArgs {
            name: name.ok_or_else(|| syn::Error::new(input.span(), "missing `name = \"...\"`"))?,
            pass: pass.ok_or_else(|| syn::Error::new(input.span(), "missing `pass = ...`"))?,
        })
    }
}

/// `name = "...", pass = ..., required = ["PART1", "PART2", ...]`
struct ComplexArgs {
    name: LitStr,
    pass: Path,
    required: ExprArray,
}

impl Parse for ComplexArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut pass: Option<Path> = None;
        let mut required: Option<ExprArray> = None;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            match ident.to_string().as_str() {
                "name" => name = Some(input.parse()?),
                "pass" => pass = Some(input.parse()?),
                "required" => required = Some(input.parse()?),
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown step_entity_complex argument: {other}"),
                    ));
                }
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(ComplexArgs {
            name: name.ok_or_else(|| syn::Error::new(input.span(), "missing `name = \"...\"`"))?,
            pass: pass.ok_or_else(|| syn::Error::new(input.span(), "missing `pass = ...`"))?,
            required: required
                .ok_or_else(|| syn::Error::new(input.span(), "missing `required = [...]`"))?,
        })
    }
}

/// Extract the impl target identifier (e.g. `DirectionHandler`) from an
/// `impl Trait for Type` block.
fn impl_target_ident(item: &ItemImpl) -> syn::Result<&syn::Ident> {
    let Type::Path(type_path) = item.self_ty.as_ref() else {
        return Err(syn::Error::new_spanned(
            &item.self_ty,
            "step_entity expects a path-typed Self (e.g. `MyHandler`)",
        ));
    };
    type_path
        .path
        .get_ident()
        .ok_or_else(|| syn::Error::new_spanned(&type_path.path, "expected single-segment ident"))
}

/// Inject `const NAME` / `const PASS_LEVEL` items at the top of the impl
/// block. Returns the modified `ItemImpl`. `pass` is the short variant
/// name (e.g. `Pass1`); the macro prefixes it with `crate::entities::PassLevel::`.
fn inject_consts(mut item: ItemImpl, name: &LitStr, pass: &Path) -> ItemImpl {
    let const_name: syn::ImplItem = syn::parse_quote! {
        const NAME: &'static str = #name;
    };
    let const_pass: syn::ImplItem = syn::parse_quote! {
        const PASS_LEVEL: crate::entities::PassLevel = crate::entities::PassLevel::#pass;
    };
    item.items.insert(0, const_pass);
    item.items.insert(0, const_name);
    item
}

#[proc_macro_attribute]
pub fn step_entity(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as SimpleArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);
    let handler_ident = match impl_target_ident(&impl_block) {
        Ok(id) => id.clone(),
        Err(e) => return e.to_compile_error().into(),
    };
    let entry_ident = format_ident!("__STEP_ENTRY_{}", handler_ident);
    let impl_with_consts = inject_consts(impl_block, &args.name, &args.pass);

    quote! {
        #impl_with_consts

        #[allow(unsafe_code, non_upper_case_globals)] // linkme uses link_section internally; ident derived from handler struct name
        #[linkme::distributed_slice(crate::entities::ENTITY_HANDLERS)]
        static #entry_ident: crate::entities::EntityHandlerEntry =
            crate::entities::EntityHandlerEntry {
                name: <#handler_ident as crate::entities::SimpleEntityHandler>::NAME,
                pass_level: <#handler_ident as crate::entities::SimpleEntityHandler>::PASS_LEVEL,
                kind: crate::entities::ReadKind::Simple {
                    read: <#handler_ident as crate::entities::SimpleEntityHandler>::read,
                },
            };
    }
    .into()
}

#[proc_macro_attribute]
pub fn step_entity_complex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as ComplexArgs);
    let impl_block = parse_macro_input!(item as ItemImpl);
    let handler_ident = match impl_target_ident(&impl_block) {
        Ok(id) => id.clone(),
        Err(e) => return e.to_compile_error().into(),
    };
    let entry_ident = format_ident!("__STEP_ENTRY_{}", handler_ident);
    let required_parts_expr: &ExprArray = &args.required;
    // ComplexEntityHandler also requires `const REQUIRED_PARTS`.
    let mut impl_with_consts = inject_consts(impl_block, &args.name, &args.pass);
    let required_const: syn::ImplItem = syn::parse_quote! {
        const REQUIRED_PARTS: &'static [&'static str] = &#required_parts_expr;
    };
    impl_with_consts.items.insert(2, required_const);

    let required_lit: Expr = syn::parse_quote! { &#required_parts_expr };

    quote! {
        #impl_with_consts

        #[allow(unsafe_code, non_upper_case_globals)] // linkme uses link_section internally; ident derived from handler struct name
        #[linkme::distributed_slice(crate::entities::ENTITY_HANDLERS)]
        static #entry_ident: crate::entities::EntityHandlerEntry =
            crate::entities::EntityHandlerEntry {
                name: <#handler_ident as crate::entities::ComplexEntityHandler>::NAME,
                pass_level: <#handler_ident as crate::entities::ComplexEntityHandler>::PASS_LEVEL,
                kind: crate::entities::ReadKind::Complex {
                    required_parts: #required_lit,
                    read: <#handler_ident as crate::entities::ComplexEntityHandler>::read_complex,
                },
            };
    }
    .into()
}
