use step_io_macros::step_entity;

trait T {}
struct A;
struct B;

#[step_entity(name = "FOO", pass = Pass1)]
impl T for (A, B) {}

fn main() {}
