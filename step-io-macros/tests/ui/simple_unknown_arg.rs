use step_io_macros::step_entity;

struct Handler;

#[step_entity(name = "FOO", pass = Pass1, bogus = 1)]
impl Handler {}

fn main() {}
