use blarg::{derive::*, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar, Switch};

#[derive(Debug, Default, BlargParser)]
struct Parameters {
    apple: usize,
    banana: bool,
    carrots: Vec<u32>,
    daikon_root: Option<String>,
}

fn main() {
    let parameters = Parameters::blarg_parse();
    println!("{parameters:?}");
}
