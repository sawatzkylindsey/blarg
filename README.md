# blarg

`blarg` is a command line argument parser, suited to this developer's opinions about what such a tool/library should look like.
Although there are many other argument parsers in Rust, I haven't quite found one that does precisely what I want.
This isn't to say the other parsers aren't good; rather, the other parsers don't suffice my particular compulsions about argument parsing.
From this, we derive "blarg":
* **arg** for argument parser.
* **bl**arg 1) because all the other names are already taken, and 2) since all the other names are already taken, why is that none of them do what *I want*? (said with no small amount of sarcasm/self-deprecation)

### CLI Design
In terms of what I believe a command line argument parser should look like, it probably makes sense to step back to what I believe is good CLI design.
As far as I know, I subscribe to the unix-like, perhaps mid-to-older school standard for CLI design.
[This guide](https://clig.dev) provides a good sketch, but I should clarify it does not wholly align with my position on CLI design.
Aside from what I think are the obvious standards (provide help via `-h`, `--help`, return `0` on success and `non-zero` on error, etc), these are my primary concerns:

* Arguments are *required* parameters to the CLI.
They are always positional.
Their order is important to the program semantics.
They are never specified via some key.
Example: `'mkdir NAME'`
* Options (aka: flags) are the *optional* parameters to the CLI.
They are specified via "single-dash single-char" or "double-dash word" keys, and may accept values or indicate flags (implicit boolean value).
Their order is not important to the program semantics.
Example: `'mkdir [-p] ..'` or `'mkdir [-m MODE] ..'` or `'diff [-i | --ignore-case] ..'`
* To re-iterate, the required parameters to a CLI are always described via positional arguments.
The optional parameters to the CLI are always described via key-value pair.
* Sometimes, arguments or options take some number of parameters.
The CLI should present this semantic clearly, and produce an error when encountering an invalid number.
* When the CLI arguments and/or options becomes too confusing to effectivley use, this is an indication that the CLI needs re-design.
There are two common re-design patterns:
    1. Use configuration file(s).
    2. Break the CLI into smaller point-focused CLIs.
       This can be done via use of separate CLI binaries, or via sub-command structure (ex: `'git add ..'` and `'git commit ..'`).

### Blarg Api
For now, I'll be happy to provide a Rust-like type-safe implementation of the above.

### Development

    cargo build
    cargo test

    ./target/debug/examples/reducer -h

