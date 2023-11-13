use blarg::{Collection, CommandLineParser, Nargs, Parameter};

fn main() {
    let mut items: Vec<u32> = Vec::default();

    let clp = CommandLineParser::new("summer");
    let parser = clp
        .add(
            Parameter::argument(Collection::new(&mut items, Nargs::AtLeastOne), "item")
                .help("The items to sum."),
        )
        .build();

    parser.parse();
    let sum: u32 = items.iter().sum();
    println!("Sum: {sum}");
}
