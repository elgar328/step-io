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
use step_io::{Attribute, RawEntity};

/// Apply per-entity non-standard normalizations in place. Mirrors the cases the
/// 2-layer reader handles in `src/reader/nonstandard.rs` (NsCase), ported to the
/// raw-entity level.
pub fn apply(map: &mut BTreeMap<u64, RawEntity>, norm: &mut Vec<&'static str>) {
    for ent in map.values_mut() {
        let RawEntity::Simple {
            name, attributes, ..
        } = ent
        else {
            continue;
        };
        match name.as_str() {
            // NS-surface-style-rendering-method: rendering_method (attr 0, a
            // required shading_surface_method enum) is `$` in most exports;
            // normalize to NORMAL_SHADING (matches the 2-layer reader).
            "SURFACE_STYLE_RENDERING" | "SURFACE_STYLE_RENDERING_WITH_PROPERTIES" => {
                if let Some(a) = attributes.get_mut(0)
                    && matches!(a, Attribute::Unset)
                {
                    *a = Attribute::Enum("NORMAL_SHADING".to_string());
                    norm.push("entity: surface_style_rendering.rendering_method<-$");
                }
            }
            // NS-psa-bare-null-style: a styles member (attr 0, SET OF
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
}
