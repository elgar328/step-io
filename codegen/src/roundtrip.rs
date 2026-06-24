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
    let src = std::fs::read_to_string(&path).expect("read file");

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
        CheckResult::Pass { validated, escaped } => {
            println!("PASS  {path}  ({validated} entities, {escaped} escaped)")
        }
        CheckResult::Fail {
            reason,
            validated,
            escaped,
        } => println!("FAIL  {path}  ({validated} entities, {escaped} escaped)\n{reason}"),
    }
}
