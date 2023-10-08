use blarg::{derive::*, CommandLineParser, Optional, Parameter, Scalar};

#[test]
fn builder_compiles() {
    CommandLineParser::new("organization");
}

#[derive(Default, BlargParser)]
struct Boo {
    asdf: Option<usize>,
    a: usize,
}

#[test]
#[ignore]
fn derive_compiles() {
    Boo::blarg_parse();
}
