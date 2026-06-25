//! Per-entity non-standard normalization (HAND-WRITTEN).
//!
//! Companion to the GENERATED `generated/generic_normalize.rs`. Two stages:
//! `generic_normalize` applies GENERIC slot-kind rules uniform across every
//! entity (req-str<-$, int->real, tagless scalar, derived->*); `entity_normalize`
//! (this file) applies PER-ENTITY fixups that cannot be a generic rule — a
//! specific entity's required field is `$`/mis-encoded and must become a specific
//! standard value, a synthetic default ref, a factor-based identification, etc.
//!
//! Runs BEFORE `generic_normalize` (so a per-entity fixup of a required ref
//! pre-empts the generic req-ref<-$ drop) and BEFORE subset/read, on the raw
//! entity map — the strict generated read only ever sees standard data. Each
//! fixup records a note into `norm`, surfaced like the generic NormCase notes.

use std::collections::BTreeMap;
use step_io::{Attribute, RawEntity, Span};

/// Apply per-entity non-standard normalizations in place: a specific entity's
/// required field is `$` or mis-encoded and must become a specific standard
/// value, a synthetic default ref, etc. — cases too entity-specific for the
/// generic slot-kind rules in `generic_normalize`.
pub fn apply(map: &mut BTreeMap<u64, RawEntity>, norm: &mut Vec<&'static str>) {
    // Synthesized entities (e.g. a placeholder COLOUR()) can't be inserted while
    // iterating the map; collect them here and insert after. Fresh ids start past
    // the current max to avoid collisions.
    let mut next_id = map.keys().max().map_or(1, |m| m + 1);
    let mut synth: Vec<(u64, RawEntity)> = Vec::new();
    for ent in map.values_mut() {
        let RawEntity::Simple {
            name, attributes, ..
        } = ent
        else {
            continue;
        };
        match name.as_str() {
            // rendering_method (attr 0, a required shading_surface_method enum) is
            // `$` or a non-standard token (real-world exports use `.UNSPECIFIED.`)
            // in some files; normalize to NORMAL_SHADING, the standard fallback for
            // an unspecified/non-standard method.
            // surface_colour (attr 1, required ref) is `$` in some files;
            // synthesize a bare COLOUR() (the schema's unspecified-colour
            // placeholder) so the rendering survives instead of being dropped by
            // the generic req-ref<-$ rule (which runs AFTER this and would delete
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
                            "CONSTANT_SHADING"
                                | "COLOUR_SHADING"
                                | "DOT_SHADING"
                                | "NORMAL_SHADING"
                        ),
                        _ => true,
                    };
                    if needs_default {
                        *a = Attribute::Enum("NORMAL_SHADING".to_string());
                        norm.push("entity: surface_style_rendering.rendering_method<-default");
                    }
                }
                if let Some(a) = attributes.get_mut(1)
                    && matches!(a, Attribute::Unset)
                {
                    let cid = next_id;
                    next_id += 1;
                    synth.push((
                        cid,
                        RawEntity::Simple {
                            id: cid,
                            name: "COLOUR".to_string(),
                            attributes: vec![],
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                column: 0,
                            },
                        },
                    ));
                    *a = Attribute::EntityRef(cid);
                    norm.push("entity: surface_style_rendering.surface_colour<-COLOUR()");
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
                            norm.push("entity: psa.styles bare-null-style->typed");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    for (id, e) in synth {
        map.insert(id, e);
    }
}
