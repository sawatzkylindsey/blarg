use blarg::{CommandLineParser, Optional, Parameter, Parser, Scalar};

#[test]
fn builder_compiles() {
    CommandLineParser::new("organization");
}

#[derive(Default, Parser)]
struct Boo {
    asdf: Option<usize>,
    a: usize,
}

#[test]
#[ignore]
fn derive_compiles() {
    Boo::parse();
}
