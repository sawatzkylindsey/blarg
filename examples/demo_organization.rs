use blarg::{Collection, CommandLineParser, GeneralParser, Nargs, Parameter, Switch};

#[derive(Debug, PartialEq, Eq)]
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
    let params = parse();
    let sum: u32 = params.items.iter().sum();
    println!("Sum: {sum}");
}

// Configure and execute the parser against `env::args`.
fn parse() -> Params {
    parse_tokens(|parser: GeneralParser| Ok(parser.parse()))
}

// Unit-testable function to configure the parser and execute it against the specified
fn parse_tokens(parse_fn: impl FnOnce(GeneralParser) -> Result<(), i32>) -> Params {
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
        .build();

    // The parse_fn signature is a `Result`.
    // However, since `GeneralParser::parse` does not return an error (it uses `std::process::exit` under the hood), the `Err` case is only reached via test.
    parse_fn(parser).expect("test-reachable-only");
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn parse_empty() {
        // Setup
        let tokens = vec![];

        // Execute & verify
        parse_tokens(|parser| parser.parse_tokens(tokens.as_slice()));
    }

    #[test]
    fn parse() {
        // Setup
        let tokens = vec!["5"];

        // Execute
        let result = parse_tokens(|parser| parser.parse_tokens(tokens.as_slice()));

        // Verify
        assert_eq!(
            result,
            Params {
                verbose: false,
                items: vec![5],
            }
        );
    }
}
