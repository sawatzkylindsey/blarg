use blarg::CommandLineParser;

fn main() {
    let ap = CommandLineParser::new("empty");
    let parser = ap.build();
    parser.parse();
    println!("empty parser");
}
