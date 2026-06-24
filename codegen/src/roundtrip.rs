//! Debug utility for the GENERATED schema-faithful module. The real gate is the
//! reference-check corpus (`CODEGEN=1`, calls `codegen::check`); this binary
//! inspects a single STEP file on disk and has no fixture dependency.
//!
//!   roundtrip <path>          # check one file: PASS/SKIP/FAIL + reason
//!   roundtrip <path> <TYPE>   # diff one entity type (attrs only) before/after

use codegen::check::{CheckResult, check_roundtrip, dump_type};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: roundtrip <path> [TYPE]");
        eprintln!("  full validation is the reference-check corpus (CODEGEN=1)");
        std::process::exit(2);
    };
    // Latin-1 corpus files exist; read bytes and lossy-convert for this debug
    // tool (the real corpus gate uses parse_bytes via check_roundtrip_bytes).
    let src = String::from_utf8_lossy(&std::fs::read(&path).expect("read file")).into_owned();

    // Type-diff mode: list the entities of TYPE that differ before/after.
    if let Some(ty) = args.next() {
        let (left, right) = dump_type(&src, &ty);
        let lset: std::collections::BTreeSet<_> = left.iter().collect();
        let rset: std::collections::BTreeSet<_> = right.iter().collect();
        println!("== only in INPUT ==");
        for s in lset.difference(&rset) {
            println!("{s}");
        }
        println!("== only in OUTPUT ==");
        for s in rset.difference(&lset) {
            println!("{s}");
        }
        return;
    }

    match check_roundtrip(&src) {
        CheckResult::Skip(why) => println!("SKIP  {path}  ({why})"),
        CheckResult::Pass {
            validated,
            escaped,
            norm,
        } => println!(
            "PASS  {path}  ({validated} entities, {escaped} escaped, {} normalized)",
            norm.len()
        ),
        CheckResult::Fail {
            reason,
            validated,
            escaped,
            norm,
        } => println!(
            "FAIL  {path}  ({validated} entities, {escaped} escaped, {} normalized)\n{reason}",
            norm.len()
        ),
    }
}
