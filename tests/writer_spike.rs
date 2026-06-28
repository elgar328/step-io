//! Per-schema writer spike (AP214e3): `write_target` projects the Universal
//! model onto a target — downgrade rename-safe subtypes, drop the rest, account
//! everything in a `LossReport`. There is no ground-truth output to diff
//! against; correctness = valid (output re-reads clean) + fully accounted.
//! Inline STEP.
//!
//! Test entities chosen from the codegen closure ∩ AP214 profile:
//! `tessellated_item` (AP242 feature) → downgrades to `geometric_representation_item`;
//! `state_observed` (illegal in AP214, no rename-safe supertype) → dropped.

use step_io::{SchemaTarget, read, write_target, write_universal};

const HEADER: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n";
const FOOTER: &str = "ENDSEC;\nEND-ISO-10303-21;\n";

fn doc(body: &str) -> String {
    format!("{HEADER}{body}{FOOTER}")
}

#[test]
fn downgrade_renames_and_drop_removes() {
    let src = doc("#1=TESSELLATED_ITEM('tess');\n#2=STATE_OBSERVED('st',$);\n");
    let (model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(
        rep.dropped.len(),
        0,
        "input should read clean: {:?}",
        rep.dropped
    );

    let (out, loss) = write_target(&model, SchemaTarget::Ap214);

    // tessellated_item -> its legal supertype keyword (rename-safe downgrade).
    assert!(out.contains("GEOMETRIC_REPRESENTATION_ITEM"), "out:\n{out}");
    assert!(
        !out.contains("TESSELLATED_ITEM"),
        "subtype keyword must be gone:\n{out}"
    );
    assert!(
        loss.downgraded
            .iter()
            .any(|(f, t)| f == "TESSELLATED_ITEM" && *t == "GEOMETRIC_REPRESENTATION_ITEM"),
        "downgraded: {:?}",
        loss.downgraded
    );

    // state_observed -> dropped (illegal, no downgrade).
    assert!(!out.contains("STATE_OBSERVED"), "out:\n{out}");
    assert!(
        loss.dropped.iter().any(|(n, _)| n == "STATE_OBSERVED"),
        "dropped: {:?}",
        loss.dropped
    );

    // header retargeted to the AP214 FILE_SCHEMA.
    assert!(
        out.contains("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }"),
        "out:\n{out}"
    );
}

#[test]
fn universal_target_equals_write_universal() {
    let src = doc("#1=TESSELLATED_ITEM('tess');\n");
    let (model, _) = read(src.as_bytes()).expect("read");
    let (out, loss) = write_target(&model, SchemaTarget::Universal);
    assert_eq!(
        out,
        write_universal(&model),
        "Universal target must equal write_universal"
    );
    assert!(loss.is_empty());
}

#[test]
fn projected_output_rereads_clean_and_is_accounted() {
    let src = doc("#1=TESSELLATED_ITEM('tess');\n#2=STATE_OBSERVED('st',$);\n");
    let (model, _) = read(src.as_bytes()).expect("read");
    let (out, loss) = write_target(&model, SchemaTarget::Ap214);

    // No ground truth — instead: the projected output is a valid target file that
    // re-reads with zero drops.
    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "projected output not clean: {:?}",
        rep2.dropped
    );

    // Accounting: 2 input entities == 1 kept(downgraded) + 1 dropped.
    assert_eq!(loss.dropped.len(), 1, "dropped: {:?}", loss.dropped);
    assert_eq!(
        loss.downgraded.len(),
        1,
        "downgraded: {:?}",
        loss.downgraded
    );
}
