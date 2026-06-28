//! Golden snapshot of Universal write output, captured BEFORE the codegen
//! `render_kw` / `dfs_render` refactor. Guards that the refactor leaves
//! Universal output byte-identical. The expected string is the pre-refactor
//! output.

use step_io::{read, write};

const DOC: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n\
#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
#2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n\
ENDSEC;\nEND-ISO-10303-21;\n";

/// Pre-refactor Universal output (captured 2026-06). Note the header is
/// hardcoded (`FILE_SCHEMA(('AUTOMOTIVE_DESIGN'))`) — `write` = Universal does
/// not retarget; that is `write_target`'s job.
const GOLDEN: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('AUTOMOTIVE_DESIGN'));\nENDSEC;\nDATA;\n#1 = APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n#2 = APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\nENDSEC;\nEND-ISO-10303-21;\n";

#[test]
fn universal_write_golden() {
    let (model, _report) = read(DOC.as_bytes()).expect("read ok");
    let out = write(&model);
    assert_eq!(
        out, GOLDEN,
        "Universal write output changed (codegen refactor regression?)"
    );
}
