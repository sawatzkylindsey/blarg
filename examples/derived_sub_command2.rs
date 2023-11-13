#[allow(unused_imports)]
use blarg::{derive::*, prelude::*, CommandLineParser, Condition, Parameter, Scalar, SubCommand};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
enum FooBar {
    Foo,
    Bar,
    Baz,
}

impl std::fmt::Display for FooBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooBar::Foo => write!(f, "foo"),
            FooBar::Bar => write!(f, "bar"),
            FooBar::Baz => write!(f, "baz"),
        }
    }
}

impl FromStr for FooBar {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "foo" => Ok(FooBar::Foo),
            "bar" => Ok(FooBar::Bar),
            "baz" => Ok(FooBar::Baz),
            _ => Err(format!("unknown: {}", value)),
        }
    }
}

#[derive(Debug, BlargParser)]
#[blarg(initializer = initial)]
struct Parameters {
    #[blarg(
        command = (FooBar::Foo, SubFoo),
        command = (FooBar::Bar, SubBar),
        help = "make a good selection ok",
    )]
    switch: FooBar,
}

impl Parameters {
    fn initial() -> Self {
        Self {
            // Doesn't matter which we chose - this is an initial that must be overwritten (by virtue of being an argument).
            switch: FooBar::Bar,
        }
    }
}

#[derive(Debug, Default, BlargSubParser)]
struct SubFoo {
    value: String,
}

impl SubFoo {
    fn initial() -> Self {
        Self::default()
    }
}

#[derive(Debug, Default, BlargSubParser)]
struct SubBar {
    #[blarg(help = "my special value")]
    value: String,
}

impl SubBar {
    fn initial() -> Self {
        Self::default()
    }
}

fn main() {
    let (parameters, sub_foo, sub_bar): (Parameters, SubFoo, SubBar) = Parameters::blarg_parse();
    println!("{parameters:?}");
    println!("{sub_foo:?}");
    println!("{sub_bar:?}");
}
