//! Per-schema writer: `write_target` projects the Universal model onto a target —
//! drop illegal entities (referential-closure cascade), account everything in a
//! `LossReport` — and sets the `FILE_SCHEMA` header AND the APD/`application_context`
//! entities to the target's values so the output is internally consistent
//! (header ↔ APD agree). There is no ground-truth output to diff against;
//! correctness = valid (output re-reads clean) + fully accounted. Inline STEP.
//!
//! step-io reads AP203/AP214/AP242 but writes a single output schema: AP242 ed2.
//! So the only projecting target is `Ap242` (drop `pre_defined_presentation_style`,
//! keep ed2-legal `state_observed`); `Universal` is the raw, non-projecting oracle.
//! (The downgrade path exists in projection but has no modeled trigger under ed2 —
//! all 12 ed2 downgrade-source entities are outside step-io's closure, so reading
//! cannot place one in the model — hence it is not exercised here.)

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
fn ap242_drops_illegal_keeps_legal_and_rereads_clean() {
    // AP242 ed2: state_observed is legal and kept; pre_defined_presentation_style
    // is illegal with no rename-safe supertype, so it is dropped (cascade-closed).
    // Input carries an AP214 header — reading is permissive; the write re-stamps ed2.
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

    // No ground truth — instead: the projected output is a valid AP242 file that
    // re-reads with zero drops.
    let (_m2, rep2) = read(out.as_bytes()).expect("re-read");
    assert_eq!(
        rep2.dropped.len(),
        0,
        "projected output not clean: {:?}",
        rep2.dropped
    );

    // Accounting: 1 illegal dropped, nothing downgraded.
    assert_eq!(loss.dropped.len(), 1, "dropped: {:?}", loss.dropped);
    assert!(
        loss.downgraded.is_empty(),
        "downgraded: {:?}",
        loss.downgraded
    );

    // header retargeted to the AP242 ed2 FILE_SCHEMA.
    assert!(
        out.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF"),
        "out:\n{out}"
    );
}

#[test]
fn universal_target_equals_write_universal() {
    let src = doc("#1=STATE_OBSERVED('st',$);\n");
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
fn header_and_apd_are_consistent_per_target() {
    // Input declares AP214-flavoured APD/AC; every write absolutely sets BOTH the
    // FILE_SCHEMA header and the APD/AC entities to the target's values.
    let body = "#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
                #2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n";
    let (mut model, _) = read(doc(body).as_bytes()).expect("read");

    // AP242 target: header AND APD both say AP242 ed2 (year normalised to 2011).
    let (ap242, _) = write_target(&mut model, SchemaTarget::Ap242);
    assert!(
        ap242.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }"),
        "{ap242}"
    );
    assert!(
        ap242.contains(
            "APPLICATION_PROTOCOL_DEFINITION('international standard',\
             'ap242_managed_model_based_3d_engineering_mim_lf',2011"
        ),
        "{ap242}"
    );
    assert!(
        ap242.contains("APPLICATION_CONTEXT('managed model based 3d engineering')"),
        "{ap242}"
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

    // Consecutive writes: absolute-set leaves no leftover — AP242 after Universal is
    // fully AP242 again (no STEPIO_UNIVERSAL residue).
    let (ap242b, _) = write_target(&mut model, SchemaTarget::Ap242);
    assert!(
        ap242b.contains("AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF { 1 0 10303 442 3 1 4 }"),
        "{ap242b}"
    );
    assert!(
        !ap242b.contains("STEPIO_UNIVERSAL"),
        "no universal leftover:\n{ap242b}"
    );
}
