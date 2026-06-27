//! Smoke test for the public codegen-generated read/write pipeline: every
//! fixture reads with full provenance accounting (nothing silently lost) and
//! re-emits to clean, count-stable text.

use step_io::{read, write};

const FIXTURES: &[(&str, &str)] = &[
    ("box_ap203", include_str!("fixtures/box_ap203.step")),
    ("box_ap214_cd", include_str!("fixtures/box_ap214_cd.step")),
    ("box_ap214_is", include_str!("fixtures/box_ap214_is.step")),
    ("box_ap242_dis", include_str!("fixtures/box_ap242_dis.step")),
    (
        "assembly_ap214_is",
        include_str!("fixtures/assembly_ap214_is.step"),
    ),
    ("cone_ap214_is", include_str!("fixtures/cone_ap214_is.step")),
    (
        "cylinder_ap214_is",
        include_str!("fixtures/cylinder_ap214_is.step"),
    ),
    (
        "ellipse_ap214_is",
        include_str!("fixtures/ellipse_ap214_is.step"),
    ),
];

#[test]
fn read_write_round_trip_is_accounted_and_clean() {
    for (name, src) in FIXTURES {
        let (model, rep) =
            read(src.as_bytes()).unwrap_or_else(|e| panic!("{name}: read failed: {e}"));

        // Provenance accounting: input + synthetic == kept + reasoned drops.
        let dropped: usize = rep.drops.values().sum();
        assert_eq!(
            rep.validated + dropped,
            rep.n_in + rep.n_synth,
            "{name}: unexplained loss (validated {} + drops {} != n_in {} + synth {})",
            rep.validated,
            dropped,
            rep.n_in,
            rep.n_synth,
        );
        assert!(rep.validated > 0, "{name}: nothing read");

        // Re-emit must reparse cleanly and be count-stable.
        let out = write(&model);
        assert!(!out.is_empty(), "{name}: empty output");
        let (_m2, r2) =
            read(out.as_bytes()).unwrap_or_else(|e| panic!("{name}: re-read failed: {e}"));
        assert_eq!(
            r2.validated, rep.validated,
            "{name}: round-trip count drift"
        );
        assert_eq!(
            r2.drops.values().sum::<usize>(),
            0,
            "{name}: output not clean (drops)"
        );
        assert_eq!(r2.n_synth, 0, "{name}: output needed synthesis");
    }
}
