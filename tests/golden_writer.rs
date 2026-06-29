//! Golden snapshot of Universal write output. Guards that the `FILE_SCHEMA` header
//! and the APD/`application_context` entities are retargeted to the non-standard
//! `STEPIO_UNIVERSAL` marker (header ↔ APD internally consistent), and that the
//! codegen `render_kw` / `dfs_render` body emission stays stable.

use step_io::{read, write_universal};

const DOC: &str = "ISO-10303-21;\nHEADER;\n\
FILE_DESCRIPTION((''),'2;1');\n\
FILE_NAME('','',(''),(''),'','','');\n\
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 214 3 1 1 }'));\n\
ENDSEC;\nDATA;\n\
#1=APPLICATION_CONTEXT('core data for automotive mechanical design processes');\n\
#2=APPLICATION_PROTOCOL_DEFINITION('international standard','automotive_design',2010,#1);\n\
ENDSEC;\nEND-ISO-10303-21;\n";

/// Universal output (captured 2026-06). The header `FILE_SCHEMA` and the APD/AC
/// entities are rewritten to the `STEPIO_UNIVERSAL` marker so the output is
/// internally consistent; the entity body is emitted by the generated writer.
const GOLDEN: &str = "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\nFILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('STEPIO_UNIVERSAL'));\nENDSEC;\nDATA;\n#1 = APPLICATION_CONTEXT('step-io universal union (non-standard, all-AP superset)');\n#2 = APPLICATION_PROTOCOL_DEFINITION('not a standard','stepio_universal',0,#1);\nENDSEC;\nEND-ISO-10303-21;\n";

#[test]
fn universal_write_golden() {
    let (mut model, _report) = read(DOC.as_bytes()).expect("read ok");
    let out = write_universal(&mut model);
    assert_eq!(
        out, GOLDEN,
        "Universal write output changed (codegen refactor regression?)"
    );
}
