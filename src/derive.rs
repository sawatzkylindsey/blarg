//! Derive Api for `blarg` configuration.
//!
//! ### Getting Started
//! Use the derive Api by starting with a parameter struct `S` instrumented with `#[derive(BlargParser)]`.
//! This will generate a function `S::blarg_parse() -> S` which parses the Cli parameters fitting `S`.
//! `blarg` will do its best to infer the intended Cli from the parameter structure `S`.
//!
//! This page includes a few demos on using the derive Api.
//! More examples are outlined in [the source](https://github.com/sawatzkylindsey/blarg/tree/main/examples).
//!
//! ```no_run
#![doc = include_str!("../examples/demo_derived.rs")]
//! ```
//!
//! This generates the following Cli program:
//! ```console
//! $ demo_derived -h
//! usage: demo_derived [-h] [--banana] [--daikon-root DAIKON_ROOT] APPLE CARROTS [...]
//! positional arguments:
//!  APPLE                                                          type: usize
//!  CARROTS [...]                                                  type: u32      initial: []
//! options:
//!  -h, --help                  Show this help message and exit.
//!  --banana
//!  --daikon-root DAIKON_ROOT                                      type: String
//! ```
//!
//!
//! ### Parser/SubParser Configuration
//! See the macro definition for details on configuring the [`BlargParser`] and [`BlargSubParser`].
//!
//! ### Parameter Configuration
//! The implicit Cli inference uses the following rules:
//! ```console
//! Type        | Parameter
//! -----------------------------------
//! Option<T>   | Parameter::option(Optional::new(..), ..)
//! Vec<T>      | Parameter::argument(Collection::new(.., Nargs::AtLeastOne), ..)
//! HashSet<T>  | Parameter::argument(Collection::new(.., Nargs::AtLeastOne), ..)
//! bool        | Parameter::option(Switch::new(..), ..)
//! T           | Parameter::argument(Scalar::new(..) , ..)
//! ```
//!
//! Notice, these implicit rules do not capture all possible `blarg` configurations.
//! Therefore, we provide the additional explicit configuration field attributes, which may be combined as necessary.
//! * `#[blarg(argument)]` or `#[blarg(option)]` to explicitly use `Parameter::argument(..)` or `Parameter::option(..)`, respectively.
//! Only one of these may be used on the same field.
//! * `#[blarg(short = C]` to explicitly set the short name for an option parameter.
//! `C` must be a char value (ex: `'c'`).
//! * `#[blarg(collection = N)]` to explicitly use `Collection::new(.., N)`, where `N` is the [Nargs](../enum.Nargs.html) variant.
//! This is useful both for non-`Vec`/`HashSet` [Collectable](../prelude/trait.Collectable.html) types, as well as to control the `Nargs` variant.
//! * `#[blarg(command = (Vi, Si), .., command = (Vj, Sj))]` to define sub-command [branches](../struct.CommandLineParser.html#method.branch) on the pairs `(Vi, Si), .., (Vj, Sj)`.
//! Each pair must be the variant `V*` and sub-parameter struct `S*` to configure.
//! `S*` must be instrumented with `#[blarg(BlargSubParser)]`, and follows the same configuration rules (both implicit and explicit) as a `BlargParser`.
//!
//! A partial example of these rules is provided as follows:
//! ```ignore
//! #[derive(Default, BlargParser)]
//! struct Parameters {
//!     #[blarg(argument)]
//!     quick: usize,
//!     // the above generates:
//!     //  .add(Parameter::argument(Scalar::new(&mut parameters.quick), "quick"))
//!
//!     #[blarg(option)]
//!     brown: usize,
//!     // the above generates:
//!     //  .add(Parameter::option(Scalar::new(&mut parameters.brown), "brown", None))
//!
//!     #[blarg(option, short = 'f')]
//!     fox: usize,
//!     // the above generates:
//!     //  .add(Parameter::option(Scalar::new(&mut parameters.fox), "fox", Some('f')))
//!
//!     #[blarg(collection = Nargs::Precisely(2))]
//!     jumps: Pair<usize>,
//!     // the above generates:
//!     //  .add(Parameter::argument(Collection::new(&mut parameters.jumps, Nargs::Precisely(2)), "jumps"))
//!     // assumes: `impl<T> Collectable<T> for Pair<T>`
//!
//!     #[blarg(command = (0, Sub0), command = (1, Sub1))]
//!     over: usize,
//!     // the above generates:
//!     //  .branch(Condition::new(Scalar::new(&mut parameters.over), "over"))
//!     //  .command(0, Sub0::setup_command)  // assuming `Sub0` is instrumented with `BlargSubParser`
//!     //  .command(1, Sub1::setup_command)  // assuming `Sub1` is instrumented with `BlargSubParser`
//! }
//!
//! #[derive(Default, BlargSubParser)]
//! struct Sub0 {
//!     ..
//! }
//!
//! #[derive(Default, BlargSubParser)]
//! struct Sub1 {
//!     ..
//! }
//! ```
//!
//! ### Help Messages
//! The previous implicit and explicit rules are sufficient to configure all possible `blarg` Cli semantics.
//! Additionally, the following field attributes may be used to configure the Cli help message.
//! * `#[blarg(help = "..")]` defines the help message for the parameter.
//! This value is passed directly via the "help" documentation mechanism ([parameter help](../struct.Parameter.html#method.help) or [condition help](../struct.Condition.html#method.help)).
//! * `#[blarg(choices)]` instructs `blarg` to use the choice function generated by instrumenting the enum struct with `#[derive(BlargChoices)]`.
//! See defining choices on a [parameter](../struct.Parameter.html#method.choice) or [condition](../struct.Condition.html#method.choice) for how this affects the Cli help message.
//! * `#[blarg(choices = F)]` instructs `blarg` to use the choice function `F`.
//! This has the same meaning as the previous point.
//!
//! The noted two `choices` attributes leverage functions of the signature `fn my_func(value: Parameter<T>) -> Parameter<T>`, where:
//! * `T` is the concrete type of the field under instrumentation.
//!
//! For example: `fn my_func(value: Parameter<usize>) -> Parameter<usize>`.
//! Notice, if `choices` is applied to a sub-command branching field (`#[blarg(command = ..)]`), then instead use `fn my_func(value: Condition<T>) -> Condition<T>`.
//!
//! A partial example of these rules is provided as follows:
//! ```ignore
//! #[derive(Default, BlargParser)]
//! struct Parameters {
//!     #[blarg(help = "do something")]
//!     lazy: usize,
//!     // the above generates:
//!     //  .add(Parameter::argument(Scalar::new(&mut parameters.lazy), "lazy")
//!     //      .help("do something"))
//!
//!     #[blarg(choices)]
//!     dog: Enumeration,
//!     // the above generates:
//!     //  .add(Enumeration::setup_choices(Parameter::argument(Scalar::new(&mut parameters.dog), "dog")))
//!     // assumes: `Enumeration` is instrumented with `BlargChoices`
//!
//!     #[blarg(choices = setup_choices)]
//!     period: usize,
//!     // the above generates:
//!     //  .add(setup_choices(Parameter::argument(Scalar::new(&mut parameters.period), "period")))
//! }
//!
//! /// My custom setup_choices fn.
//! fn setup_choices(value: Parameter<usize>) -> Parameter<usize> {
//!     value.choice(0, "the 0th choice")
//!         .choice(1, "the 1st choice")
//!         .choice(2, "the 2nd choice")
//! }
//!
//! #[derive(BlargChoices)]
//! enum Enumeration {
//!     ..
//! }
//! ```
//!
//! ### Choices
//! In the case of enums, simply instrument with `#[derive(BlargChoices)]` to automatically generate the setup function.
//! The enum may be configured with the following field attributes:
//! * `#[blarg(help = "..")]` defines the help message for the variant.
//! * `#[blarg(hidden)]` instructs `blarg` to hide the variant.
//!
//! For example:
//! ```ignore
//! #[derive(BlargChoices)]
//! enum Enumeration {
//!     VariantA,
//!     // the above generates:
//!     //  .choice(VariantA, "")
//!
//!     #[blarg(help = "the variant B choice")]
//!     VariantB,
//!     // the above generates:
//!     //  .choice(VariantB, "the variant B choice")
//!
//!     #[blarg(hidden)]
//!     VariantC,
//!     // the above does *not* instrument a `.choice(..)`
//! }
//! ```

pub use blarg_derive::*;
