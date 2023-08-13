use blarg::{CommandLineParser, Parser};

#[test]
fn abc() {
    CommandLineParser::new("organization");
}

#[derive(Default, Parser)]
struct Boo;

#[test]
#[ignore]
fn boo() {
    let _boo = Boo::parse();
}
