use step_io_macros::step_entity_complex;

struct Handler;

#[step_entity_complex(name = "FOO", pass = Pass1)]
impl Handler {}

fn main() {}
