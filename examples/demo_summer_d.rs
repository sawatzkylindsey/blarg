use blarg::{derive::*, Collection, CommandLineParser, Nargs, Parameter};

#[derive(Default, BlargParser)]
#[blarg(program = "summer")]
struct Parameters {
    #[blarg(help = "The items to sum.")]
    item: Vec<u32>,
}

fn main() {
    let parameters = Parameters::blarg_parse();
    let sum: u32 = parameters.item.iter().sum();
    println!("Sum: {sum}");
}
