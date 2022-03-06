# blarg

`blarg` is a command line argument parser, suited to this developer's opinions about what such a tool/library should look like.
Although there are many other argument parsers in Rust, I haven't quite found one that does precisely what I want.
This isn't to say the other parsers aren't good; rather, the other parsers don't suffice my particular compulsions about argument parsing.
From this, we derive "blarg":
* **arg** for argument parser.
* **bl**arg 1) because all the other names are already taken, and 2) since all the other names are already taken, why is that none of them do what *I want*? (said with no small amount of sarcasm/self-deprecation)

So then, what should a command line argument parser look like?
As far as I know, I subscribe to the de-factor unix-like standard for CLI design, from which argument parser structure follows.
I haven't read [this whole guide](https://clig.dev), but a quick skim seems to align with my position.
As an interesting aside, that guide encourages using a library, mentioning two such Rust libraries - both of which do not satisfy other interface level paradigms stated in that guide.
Anyways, aside from what I think are the obvious standards (provide help via `-h`, `--help`, return `0` on success and `non-zero` on error, etc), these are my primary concerns:

* Arguments are *required* parameters to the CLI.
They are always positional.
Their order is important to the program semantics.
They are never specified via some key.
Example: `mkdir NAME`
* Options are the *optional* parameters to the CLI.
They are specified via "single-dash single-char" or "double-dash word" keys, and may accept values or indicate flags (implicit bool value).
Their order is not important to the program semantics.
Example: `mkdir [-p] ..` or `mkdir [-m MODE] ..` or `diff [-i | --ignore-case] ..`

Honestly, if any of the Rust libraries adhered to this requirements I could probably live with any fallout idiosyncrasies.
None that I have found do that, or at least do that by default (I'll admit, perhaps there is some flag I haven't found).

I may come back to add more requirements in the future.
For now, I'll be happy to provide a Rust-like type-safe implementation of the above.

