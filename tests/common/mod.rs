//! Shared by the integration tests.

use step_io::StepModel;

/// The model as Part 21 text, through [`step_io::write`].
pub fn text(model: &StepModel) -> String {
    let mut out = Vec::new();
    step_io::write(model, &mut out).expect("writing to a Vec does not fail");
    String::from_utf8(out).expect("Part 21 output is ASCII")
}
