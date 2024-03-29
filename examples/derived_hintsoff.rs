#[allow(unused_imports)]
use blarg::{
    derive::*, prelude::*, Collection, CommandLineParser, Nargs, Optional, Parameter, Scalar,
    Switch,
};

#[derive(Debug, Default, BlargParser)]
#[blarg(hints_off)]
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
    #[blarg(argument, collection = Nargs::Precisely(2))]
    halwa_puri: Pair<String>,
}

#[derive(Default)]
struct Pair<T: std::fmt::Debug> {
    left: Option<T>,
    right: Option<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Pair<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let l: String = self
            .left
            .as_ref()
            .map(|v| format!("{v:?}"))
            .unwrap_or("".to_string());
        let r: String = self
            .right
            .as_ref()
            .map(|v| format!("{v:?}"))
            .unwrap_or("".to_string());
        write!(f, "({l}, {r})")
    }
}

impl<T: std::fmt::Debug> Collectable<T> for Pair<T> {
    fn add(&mut self, item: T) -> Result<(), String> {
        if self.left.is_none() {
            self.left.replace(item);
        } else if self.right.is_none() {
            self.right.replace(item);
        }

        // We don't need `Pair` to be fallible because we're using `Nargs::Precisely(2)`.
        Ok(())
    }
}

fn main() {
    let parameters: Parameters = Parameters::blarg_parse();
    println!("{parameters:?}");
}
