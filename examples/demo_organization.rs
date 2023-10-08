use blarg::{Collection, CommandLineParser, Nargs, Parameter, Switch};

pub struct Params {
    verbose: bool,
    items: Vec<u32>,
}

impl Params {
    fn init() -> Self {
        Self {
            verbose: false,
            items: Vec::default(),
        }
    }
}

fn main() {
    let mut params = Params::init();

    let clp = CommandLineParser::new("organization");
    let parser = clp
        .add(Parameter::option(
            Switch::new(&mut params.verbose, true),
            "verbose",
            Some('v'),
        ))
        .add(Parameter::argument(
            Collection::new(&mut params.items, Nargs::AtLeastOne),
            "item",
        ))
        .build()
        .expect("The parser configuration must be valid (ex: no duplicate parameter names).");

    parser.parse();
    let sum: u32 = params.items.iter().sum();
    println!("Sum: {sum}");
}
