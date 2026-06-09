use step_io_macros::step_entity_complex;

struct Handler;

#[step_entity_complex(name = "FOO", required = ["PART"])]
impl Handler {}

fn main() {}
