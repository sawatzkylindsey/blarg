#[allow(unused_imports)]
use blarg::{
    BlargParser, BlargSubParser, CommandLineParser, Condition, Parameter, Scalar, SubCommand,
};
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
enum FooBar {
    Foo,
    Bar,
}

impl std::fmt::Display for FooBar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooBar::Foo => write!(f, "foo"),
            FooBar::Bar => write!(f, "bar"),
        }
    }
}

impl FromStr for FooBar {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "foo" => Ok(FooBar::Foo),
            "bar" => Ok(FooBar::Bar),
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
        help = "make a good selection ok"
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
    value: String,
}

impl SubBar {
    fn initial() -> Self {
        Self::default()
    }
}

fn main() {
    let parameters = Parameters::parse();
    println!("{parameters:?}");
}
