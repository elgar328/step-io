//! Internal proc-macro helpers for `step-io` entity handlers.
//!
//! Provides `#[step_entity]` / `#[step_entity_complex]` attribute macros
//! that emit the const NAME member + `ENTITY_HANDLERS` registry entry
//! that every handler module would otherwise hand-roll.
//!
//! Path policy: macro output uses `crate::...` relative paths so it
//! resolves against the caller crate (`step-io`). step-io-macros must not
//! reference step-io directly (would be a circular dependency).

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, ExprArray, Fields, ItemImpl, LitStr, Token, Type, parse::Parse,
    parse::ParseStream, parse_macro_input,
};

/// `name = "ENTITY_NAME"[, is_2d]`
struct SimpleArgs {
    name: LitStr,
    is_2d: bool,
}

impl Parse for SimpleArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut is_2d = false;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                // Flag form (no `= value`): marks a 2D parameter-space handler.
                "is_2d" => is_2d = true,
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse()?);
                }
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
            is_2d,
        })
    }
}

/// `name = "...", pass = ..., cases = [["PART1","PART2"], ["PART1","PART3"]]`
/// — every exact part-set this complex handler claims. An instance matches iff
/// its distinct part-set EQUALS one listed case (set equality).
struct ComplexArgs {
    name: LitStr,
    cases: ExprArray,
    is_2d: bool,
}

impl Parse for ComplexArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<LitStr> = None;
        let mut cases: Option<ExprArray> = None;
        let mut is_2d = false;
        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            match ident.to_string().as_str() {
                // Flag form (no `= value`): marks a 2D parameter-space handler.
                "is_2d" => is_2d = true,
                "name" => {
                    input.parse::<Token![=]>()?;
                    name = Some(input.parse()?);
                }
                "cases" => {
                    input.parse::<Token![=]>()?;
                    cases = Some(input.parse()?);
                }
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
            cases: cases
                .ok_or_else(|| syn::Error::new(input.span(), "missing `cases = [[...], ...]`"))?,
            is_2d,
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

/// Inject the `const NAME` item at the top of the impl block. Returns the
/// modified `ItemImpl`.
fn inject_consts(mut item: ItemImpl, name: &LitStr) -> ItemImpl {
    let const_name: syn::ImplItem = syn::parse_quote! {
        const NAME: &'static str = #name;
    };
    item.items.insert(0, const_name);
    item
}

/// Derive reader `resolve_select` / writer `emit_select` for a "simple" STEP
/// `SELECT` enum — one whose every variant is a single-tuple `Variant(XId)`
/// over an arena-id newtype. The member list then lives in exactly one place
/// (the enum), so the reader's sequential id-map probes and the writer's emit
/// match can never drift out of sync.
///
/// Generates an inherent impl with two `pub(crate)` methods:
/// - `resolve_select<C: IdResolver>(ctx, file_id) -> Option<Self>` — probes
///   each member arena in declaration order, returning the first hit.
/// - `emit_select<C: StepResolver>(&self, buf) -> u64` — matches the variant
///   and returns its emitted STEP id.
///
/// The two seams [`IdResolver`](crate::ir::select::IdResolver) /
/// [`StepResolver`](crate::ir::select::StepResolver) keep this generated code
/// free of direct `reader`/`writer` type names. A variant that is not a
/// single-field tuple is a compile error (the enum is not a simple SELECT).
#[proc_macro_derive(StepSelect)]
pub fn derive_step_select(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match step_select_impl(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn step_select_impl(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_ident = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "StepSelect can only be derived for enums",
        ));
    };

    // Collect (variant ident, single tuple field type) for each variant,
    // rejecting any variant that is not exactly `Variant(SomeId)`.
    let mut members: Vec<(&syn::Ident, &Type)> = Vec::new();
    for variant in &data.variants {
        let Fields::Unnamed(fields) = &variant.fields else {
            return Err(syn::Error::new_spanned(
                variant,
                "StepSelect requires every variant to be a single-tuple `Variant(XId)`",
            ));
        };
        if fields.unnamed.len() != 1 {
            return Err(syn::Error::new_spanned(
                variant,
                "StepSelect requires exactly one field per variant",
            ));
        }
        members.push((&variant.ident, &fields.unnamed[0].ty));
    }

    let resolve_arms = members.iter().map(|(vident, ty)| {
        quote! {
            if let ::core::option::Option::Some(id) =
                <C as crate::ir::select::IdResolver>::resolve_arena_id::<#ty>(ctx, file_id)
            {
                return ::core::option::Option::Some(Self::#vident(id));
            }
        }
    });
    let emit_arms = members.iter().map(|(vident, _ty)| {
        quote! {
            Self::#vident(id) => <C as crate::ir::select::StepResolver>::step_of(buf, *id),
        }
    });

    Ok(quote! {
        impl #enum_ident {
            /// Resolve a STEP file id (`#N`) to this SELECT by probing each
            /// member arena in declaration order. Generated by `StepSelect`.
            pub(crate) fn resolve_select<C: crate::ir::select::IdResolver>(
                ctx: &C,
                file_id: u64,
            ) -> ::core::option::Option<Self> {
                #(#resolve_arms)*
                ::core::option::Option::None
            }

            /// Emit this SELECT's target STEP id. Generated by `StepSelect`.
            pub(crate) fn emit_select<C: crate::ir::select::StepResolver>(&self, buf: &C) -> u64 {
                match self {
                    #(#emit_arms)*
                }
            }
        }
    })
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
    let is_2d = args.is_2d;
    let impl_with_consts = inject_consts(impl_block, &args.name);

    quote! {
        #impl_with_consts

        #[allow(unsafe_code, non_upper_case_globals)] // linkme uses link_section internally; ident derived from handler struct name
        #[linkme::distributed_slice(crate::entities::ENTITY_HANDLERS)]
        static #entry_ident: crate::entities::EntityHandlerEntry =
            crate::entities::EntityHandlerEntry {
                name: <#handler_ident as crate::entities::SimpleEntityHandler>::NAME,
                is_2d: #is_2d,
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
    let is_2d = args.is_2d;
    let impl_with_consts = inject_consts(impl_block, &args.name);
    // Build `&[&["A","B"], &["C","D"], ...]` (type `&[&[&str]]`). Each inner
    // case array must be referenced into a slice — they differ in length, so a
    // plain nested array literal would not type-check.
    let case_slices: Vec<Expr> = args
        .cases
        .elems
        .iter()
        .map(|inner| syn::parse_quote! { &#inner })
        .collect();
    let cases_lit: Expr = syn::parse_quote! { &[ #(#case_slices),* ] };

    quote! {
        #impl_with_consts

        #[allow(unsafe_code, non_upper_case_globals)] // linkme uses link_section internally; ident derived from handler struct name
        #[linkme::distributed_slice(crate::entities::ENTITY_HANDLERS)]
        static #entry_ident: crate::entities::EntityHandlerEntry =
            crate::entities::EntityHandlerEntry {
                name: <#handler_ident as crate::entities::ComplexEntityHandler>::NAME,
                is_2d: #is_2d,
                kind: crate::entities::ReadKind::Complex {
                    cases: #cases_lit,
                    read: <#handler_ident as crate::entities::ComplexEntityHandler>::read_complex,
                },
            };
    }
    .into()
}
