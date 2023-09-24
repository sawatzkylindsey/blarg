#[allow(unused_imports)]
use blarg::{
    BlargParser, Collectable, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar,
    Switch,
};

#[derive(Debug, BlargParser)]
#[blarg(program = "edgy", initializer = initial)]
struct Parameters {
    #[blarg(option)]
    items: Vec<usize>,
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

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
