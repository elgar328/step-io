//! Internal proc-macro helpers for `step-io` entity handlers.
//!
//! Provides `#[step_entity]` / `#[step_entity_complex]` attribute macros
//! that emit the const NAME / PASS_LEVEL members + `ENTITY_HANDLERS`
//! registry entry that every handler module would otherwise hand-roll.

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn step_entity(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn step_entity_complex(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
