//! Per-entity non-standard normalization (HAND-WRITTEN).
//!
//! Companion to the GENERATED `generated/generic_normalize.rs`. Two stages:
//! `generic_normalize` applies GENERIC slot-kind rules uniform across every
//! entity (missing-required fixups, int->real coercion, bare-scalar tagging,
//! derived-slot reset); `entity_normalize`
//! (this file) applies PER-ENTITY fixups that cannot be a generic rule — a
//! specific entity's required field is `$`/mis-encoded and must become a specific
//! standard value or a synthetic default ref.
//!
//! Runs BEFORE `generic_normalize` (so a per-entity fixup of a required ref
//! pre-empts the generic required-reference-missing drop) and BEFORE the
//! generated read, on each raw entity as it is read — the strict generated read
//! only ever sees standard data. Each fixup records a note, surfaced in
//! `Report::normalized` like the generic rules' notes.

use crate::parser::{Attribute, RawEntity};

/// An entity a fixup adds to the model, and the slot of the fixed entity that
/// references it. The reader numbers it once every input id is known.
pub(crate) enum Synth {
    /// A bare `COLOUR()`.
    Colour { slot: usize },
}

/// Apply per-entity non-standard normalizations to `ent` in place: a specific
/// entity's required field is `$` or mis-encoded and must become a specific
/// standard value, a synthetic default ref, etc. — cases too entity-specific
/// for the generic slot-kind rules in `generic_normalize`. Returns the
/// synthetic entity the fix references, if it added one; the slot then holds a
/// placeholder reference.
pub(crate) fn normalize_entity(ent: &mut RawEntity, norm: &mut Vec<&'static str>) -> Option<Synth> {
    let RawEntity::Simple {
        name, attributes, ..
    } = ent
    else {
        return None;
    };
    let mut synth = None;
    match name.as_str() {
        // rendering_method (attr 0, a required shading_surface_method enum) is
        // `$` or a non-standard token (real-world exports use `.UNSPECIFIED.`)
        // in some files; normalize to NORMAL_SHADING, the standard fallback for
        // an unspecified/non-standard method.
        // surface_colour (attr 1, required ref) is `$` in some files;
        // synthesize a bare COLOUR() (the schema's unspecified-colour
        // placeholder) so the rendering survives instead of being dropped by
        // the generic required-reference-missing rule (which runs AFTER this and would delete
        // the whole surface_style_rendering).
        "SURFACE_STYLE_RENDERING" | "SURFACE_STYLE_RENDERING_WITH_PROPERTIES" => {
            if let Some(a) = attributes.get_mut(0) {
                // `$` OR a non-standard enum token (real-world exports use
                // `.UNSPECIFIED.`, not a valid shading_surface_method value) ->
                // the standard NORMAL_SHADING. The four valid tokens are below;
                // anything else (incl. `.UNSPECIFIED.`) is normalized.
                let needs_default = match a {
                    Attribute::Enum(s) => !matches!(
                        s.as_str(),
                        "CONSTANT_SHADING" | "COLOUR_SHADING" | "DOT_SHADING" | "NORMAL_SHADING"
                    ),
                    _ => true,
                };
                if needs_default {
                    *a = Attribute::Enum("NORMAL_SHADING".to_string());
                    norm.push("missing or non-standard rendering method set to NORMAL_SHADING");
                }
            }
            if let Some(a @ Attribute::Unset) = attributes.get_mut(1) {
                *a = Attribute::EntityRef(0);
                synth = Some(Synth::Colour { slot: 1 });
                norm.push("missing surface colour set to an unspecified COLOUR");
            }
        }
        // a styles member (attr 0, SET OF
        // presentation_style_select) written as a bare enum `.NULL.`
        // (null_style) instead of the standard typed `NULL_STYLE(.NULL.)`.
        // The only enum member of presentation_style_select is null_style, so
        // any bare enum here is it; wrap to the typed form the select read
        // expects.
        "PRESENTATION_STYLE_ASSIGNMENT" => {
            if let Some(Attribute::List(styles)) = attributes.get_mut(0) {
                for s in styles.iter_mut() {
                    if matches!(s, Attribute::Enum(_)) {
                        let val = std::mem::replace(s, Attribute::Unset);
                        *s = Attribute::Typed {
                            type_name: "NULL_STYLE".to_string(),
                            value: Box::new(val),
                        };
                        norm.push("bare null style written as NULL_STYLE");
                    }
                }
            }
        }
        _ => {}
    }
    synth
}
