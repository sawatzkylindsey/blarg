#[allow(unused_imports)]
use blarg::{
    derive::*, prelude::*, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar,
    Switch,
};

#[derive(Debug, Default, BlargParser)]
struct Parameters {
    #[blarg(help = "just apple things")]
    apple: usize,
    #[blarg(help = "activate 'banana split' mode")]
    banana_split: bool,
    #[blarg(option, short = 'c')]
    cucumber: Option<usize>,
    #[blarg(option, help = "abc 123")]
    daikon_root: Vec<usize>,
    #[blarg(option, short = 'e')]
    edamame: usize,
    #[blarg(option)]
    falafel: bool,
    #[blarg(argument)]
    gateau: Vec<String>,
    #[blarg(argument, collection = Nargs::Any)]
    halwa_puri: Pair<String>,
}

#[derive(Debug, Default)]
struct Pair<T> {
    left: Option<T>,
    right: Option<T>,
}

impl<T> Collectable<T> for Pair<T> {
    fn add(&mut self, item: T) {
        if self.left.is_none() {
            self.left.replace(item);
        } else if self.right.is_none() {
            self.right.replace(item);
        }
    }
}

fn main() {
    let parameters = Parameters::blarg_parse();
    println!("{parameters:?}");
}
