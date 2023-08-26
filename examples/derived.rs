use blarg::{BlargParser, CommandLineParser, Optional, Parameter, Scalar};

#[derive(Debug, Default, BlargParser)]
#[blarg(program = "abc")]
struct Parameters {
    a: usize,
    b: usize,
    c: Option<usize>,
}

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
