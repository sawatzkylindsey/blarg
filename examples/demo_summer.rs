use blarg::{Collection, CommandLineParser, Nargs, Parameter};

fn main() {
    let mut items: Vec<u32> = Vec::default();

    let clp = CommandLineParser::new("summer");
    let parser = clp
        .add(
            Parameter::argument(Collection::new(&mut items, Nargs::AtLeastOne), "item")
                .help("The items to sum."),
        )
        .build()
        .expect("The parser configuration must be valid (ex: no parameter name repeats).");

    parser.parse();
    let sum: u32 = items.iter().sum();
    println!("Sum: {sum}");
}
