//! Per-schema writer: `write_target` projects the Universal model onto a target —
//! downgrade rename-safe subtypes, drop the rest (referential-closure cascade),
//! account everything in a `LossReport` — and sets the `FILE_SCHEMA` header AND the
//! APD/`application_context` entities to the target's values so the output is
//! internally consistent (header ↔ APD agree). There is no ground-truth output to
//! diff against; correctness = valid (output re-reads clean) + fully accounted.
//! Inline STEP.
//!
//! Coverage: AP214/AP203 (`tessellated_item` → `geometric_representation_item`
//! downgrade; `state_observed` drop), AP242 superset (`state_observed` legal/kept,
//! `pre_defined_presentation_style` drop), and header ↔ APD consistency per target.

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
    let (mut model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(
        rep.dropped.len(),
        0,
        "input should read clean: {:?}",
        rep.dropped
    );

    let (out, loss) = write_target(&mut model, SchemaTarget::Ap214);

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
    let (mut model, _) = read(src.as_bytes()).expect("read");
    let (out, loss) = write_target(&mut model, SchemaTarget::Universal);
    assert_eq!(
        out,
        write_universal(&mut model),
        "Universal target must equal write_universal"
    );
    assert!(loss.is_empty());
}

#[test]
fn projected_output_rereads_clean_and_is_accounted() {
    let src = doc("#1=TESSELLATED_ITEM('tess');\n#2=STATE_OBSERVED('st',$);\n");
    let (mut model, _) = read(src.as_bytes()).expect("read");
    let (out, loss) = write_target(&mut model, SchemaTarget::Ap214);

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

#[test]
fn ap203_downgrade_and_drop() {
    // AP203 behaves like AP214 for these: tessellated_item downgrades, state_observed drops.
    let src = doc("#1=TESSELLATED_ITEM('tess');\n#2=STATE_OBSERVED('st',$);\n");
    let (mut model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(rep.dropped.len(), 0, "input clean: {:?}", rep.dropped);

    let (out, loss) = write_target(&mut model, SchemaTarget::Ap203);

    assert!(out.contains("GEOMETRIC_REPRESENTATION_ITEM"), "out:\n{out}");
    assert!(!out.contains("TESSELLATED_ITEM"), "out:\n{out}");
    assert!(!out.contains("STATE_OBSERVED"), "out:\n{out}");
    assert!(
        loss.downgraded
            .iter()
            .any(|(f, t)| f == "TESSELLATED_ITEM" && *t == "GEOMETRIC_REPRESENTATION_ITEM"),
        "downgraded: {:?}",
        loss.downgraded
    );
    assert!(
        loss.dropped.iter().any(|(n, _)| n == "STATE_OBSERVED"),
        "dropped: {:?}",
        loss.dropped
    );

    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "projected output not clean: {:?}",
        rep2.dropped
    );
    assert_eq!(loss.dropped.len(), 1, "dropped: {:?}", loss.dropped);
    assert_eq!(
        loss.downgraded.len(),
        1,
        "downgraded: {:?}",
        loss.downgraded
    );

    // AP203 header.
    assert!(
        out.contains(
            "AP203_CONFIGURATION_CONTROLLED_3D_DESIGN_OF_MECHANICAL_PARTS_AND_ASSEMBLIES_MIM_LF"
        ),
        "out:\n{out}"
    );
}

#[test]
fn ap242_superset_drops_illegal_keeps_legal() {
    // AP242 is a superset: state_observed is legal here (illegal in AP214/AP203),
    // while pre_defined_presentation_style is illegal with no rename-safe supertype.
    let src = doc("#1=PRE_DEFINED_PRESENTATION_STYLE('pps');\n#2=STATE_OBSERVED('st',$);\n");
    let (mut model, rep) = read(src.as_bytes()).expect("read");
    assert_eq!(rep.dropped.len(), 0, "input clean: {:?}", rep.dropped);

    let (out, loss) = write_target(&mut model, SchemaTarget::Ap242);

    assert!(
        out.contains("STATE_OBSERVED"),
        "AP242-legal entity must survive:\n{out}"
    );
    assert!(
        !out.contains("PRE_DEFINED_PRESENTATION_STYLE"),
        "illegal entity must be dropped:\n{out}"
    );
    assert!(
        loss.dropped
            .iter()
            .any(|(n, _)| n == "PRE_DEFINED_PRESENTATION_STYLE"),
        "dropped: {:?}",
        loss.dropped
    );

    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "projected output not clean: {:?}",
        rep2.dropped
    );
    assert_eq!(loss.dropped.len(), 1, "dropped: {:?}", loss.dropped);
    assert!(
        loss.downgraded.is_empty(),
        "downgraded: {:?}",
        loss.downgraded
    );

    // AP242 header.
    assert!(
        out.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF"),
        "out:\n{out}"
    );
}

#[test]
fn header_and_apd_are_consistent_per_target() {
    // Input declares AP214-flavoured APD/AC; every write absolutely sets BOTH the
    // FILE_SCHEMA header and the APD/AC entities to the target's values.
    let body = "#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
                #2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n";
    let (mut model, _) = read(doc(body).as_bytes()).expect("read");

    // AP214 target: header AND APD both say AP214 (year normalised to the profile's 2009).
    let (ap214, _) = write_target(&mut model, SchemaTarget::Ap214);
    assert!(
        ap214.contains("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }"),
        "{ap214}"
    );
    assert!(
        ap214.contains(
            "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2009"
        ),
        "{ap214}"
    );
    assert!(
        ap214.contains("APPLICATION_CONTEXT('Core Data for Automotive Mechanical Design Process')"),
        "{ap214}"
    );

    // Universal: header AND APD both the non-standard STEPIO_UNIVERSAL marker.
    let uni = write_universal(&mut model);
    assert!(uni.contains("FILE_SCHEMA(('STEPIO_UNIVERSAL'))"), "{uni}");
    assert!(
        uni.contains("APPLICATION_PROTOCOL_DEFINITION('not a standard','stepio_universal',0"),
        "{uni}"
    );
    assert!(
        uni.contains(
            "APPLICATION_CONTEXT('step-io universal union (non-standard, all-AP superset)')"
        ),
        "{uni}"
    );

    // Consecutive writes: absolute-set leaves no leftover — AP214 after Universal is
    // fully AP214 again (no STEPIO_UNIVERSAL residue).
    let (ap214b, _) = write_target(&mut model, SchemaTarget::Ap214);
    assert!(
        ap214b.contains("AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }"),
        "{ap214b}"
    );
    assert!(
        ap214b.contains(
            "APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2009"
        ),
        "{ap214b}"
    );
    assert!(
        !ap214b.contains("STEPIO_UNIVERSAL"),
        "no universal leftover:\n{ap214b}"
    );
}
