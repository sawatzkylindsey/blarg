use blarg::{CommandLineParser, Optional, Parameter, Parser, Scalar};

#[derive(Debug, Default, Parser)]
#[parser(program = "abc")]
struct Parameters {
    a: usize,
    b: usize,
    c: Option<usize>,
}

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
