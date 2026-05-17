use step_io_macros::step_entity;

mod inner {
    pub struct Handler;
}

#[step_entity(name = "FOO", pass = Pass1)]
impl inner::Handler {}

fn main() {}
