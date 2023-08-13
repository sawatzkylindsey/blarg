use blarg::{CommandLineParser, Parameter, Parser, Scalar};

#[derive(Debug, Default, Parser)]
struct Parameters {
    a: usize,
    b: usize,
}

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
