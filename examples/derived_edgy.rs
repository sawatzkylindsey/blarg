#[allow(unused_imports)]
use blarg::{
    derive::*, prelude::*, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar,
    Switch,
};

#[derive(Debug, BlargParser)]
#[blarg(program = "edgy", initializer = initial)]
struct Parameters {
    #[blarg(option)]
    items: Vec<usize>,
    #[blarg(choices = elective_choices)]
    elective: Option<usize>,
    discretionary: Option<usize>,
}

impl Parameters {
    fn initial() -> Self {
        Self {
            items: vec![1, 2, 3],
            elective: Some(4),
            discretionary: None,
        }
    }
}

fn elective_choices(value: Parameter<usize>) -> Parameter<usize> {
    value
        .choice(0, "0th")
        .choice(1, "1st")
        .choice(2, "2nd")
        .choice(3, "3rd")
}

fn main() {
    let parameters = Parameters::blarg_parse();
    println!("{parameters:?}");
}
