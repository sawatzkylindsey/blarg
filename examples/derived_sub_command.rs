#[allow(unused_imports)]
use blarg::{
    BlargParser, BlargSubParser, CommandLineParser, Condition, Parameter, Scalar, SubCommand,
};

#[derive(Debug, Default, BlargParser)]
struct Parameters {
    #[blarg(command = (0, SubA), command = (1, SubB))]
    switch: usize,
}

#[derive(Debug, Default, BlargSubParser)]
struct SubA {
    value: String,
}

#[derive(Debug, Default, BlargSubParser)]
struct SubB {
    value: String,
}

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
