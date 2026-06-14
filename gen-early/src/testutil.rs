//! Shared test fixtures: the real `schema/early.toml` loader and `Ctx` /
//! synth-select helpers used by the per-module test suites.

#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use crate::classify::Ctx;
use crate::emit::emit_select_synth;
use crate::mapping::Mapping;
use crate::schema::EarlyToml;

/// The real `schema/early.toml` — the authority the generator reads.
pub(crate) fn schema() -> EarlyToml {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    toml::from_str(
        &std::fs::read_to_string(format!("{root}/schema/early.toml")).expect("read early.toml"),
    )
    .expect("parse early.toml")
}

pub(crate) fn ctx_from(schema: EarlyToml) -> Ctx {
    Ctx {
        schema,
        mapping: Mapping {
            generate: vec![],
            generate_all: false,
            serialize_with_id: Vec::new(),
            enums: BTreeMap::new(),
            selects: BTreeMap::new(),
            derived: BTreeMap::new(),
            read_only: Vec::new(),
        },
    }
}

pub(crate) fn empty_ctx() -> Ctx {
    ctx_from(EarlyToml {
        entity: BTreeMap::new(),
        types: BTreeMap::new(),
    })
}

pub(crate) fn synth_select(sel: &str) -> (String, String, String) {
    let ctx = ctx_from(schema());
    let (mut m, mut b, mut s) = (String::new(), String::new(), String::new());
    emit_select_synth(
        &ctx,
        sel,
        &mut m,
        &mut b,
        &mut s,
        &mut BTreeSet::new(),
        &mut BTreeSet::new(),
    );
    (m, b, s)
}
