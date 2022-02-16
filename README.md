# seatrial: situational-mock-based load testing

`seatrial` is a load generation tool for HTTP/1.1 services built to simulate
traffic on known-common flows, particularly in monolith-ish applications. It
operates under the model of: one or more Situations are executed in parallel,
involving one or more Grunts who will go through the flows of using the backing
service(s) based on the rules defined in their associated Persona. This makes
`seatrial` a decent fit for testing otherwise constant-traffic applications'
behaviors under fairly-predictably-bursty load.

This tool is technically capable of performing tests like "just attack this
endpoint until it falls over", but is not designed around them. It's also not
(yet?) well-suited to unpredictable loads - it can probably be used in a
fuzzing manner to discover such breaking points, but `seatrial` is currently
optimized for taking historical learnings plus data gleamed from the rest of
your observability stack, and preventing repeats of the same outages (and
indeed, for helping developers make such scale events, Non-Events).

## Usage

> For further detail and commentary, see `man 1 seatrial`, or
> `manual/seatrial.1.scd` in the source tree. While you're at it, there's also the
> following other manual pages, accessible via the same pattern:
>
> - `seatrial(5)`
> - `seatrial.lua(3)`

```
Usage: seatrial <base_url> <req_situation> [<situations...>] [-m <multiplier>]

situational-mock-based load testing

Positional Arguments:
  base_url          base URL for all situations in this run
  req_situation     path to a RON file in seatrial(5) situation config format
  situations        optional paths to additional RON files in seatrial(5)
                    situation config format

Options:
  -m, --multiplier  integral multiplier for grunt counts (minimum 1)
  --help            display usage information
```

## Development and Packaging

Minimum Supported Rust Version is 1.58, as specified in `Cargo.toml`.
Compilation requires nothing particularly special on the host OS beyond a
standard Rust compiler stack; see `Dockerfile` for an example build.
Documentation is in [scdoc
format](https://git.sr.ht/~sircmpwn/scdoc/tree/master/item/scdoc.5.scd), and
should be compiled to roff by packagers (if you further process the roff into
HTML or GNU Info or whatever, cool, but at least ship the manual pages).

The source must pass `rustfmt` and `clippy` without errors. It _should_ also
pass without warnings, unless there's good reason to leave the warnings in
place.

MacOS users can use `brew bundle` to install the development dependencies as
found in `Brewfile` (note that this will install Docker Desktop via its Cask;
if you've installed Docker Desktop via some other means you may need to either
uninstall your existing copy, or remove this line from `Brewfile`. Take
care not to submit a PR with such a change in place!). Linux users should
install `rustup` and `scdoc` via their distribution package manager. If
`rustup` isn't available but a Rust of at least the MSRV is, then
system-wide Rust is fine.

If you've never worked with Rust before and `rustup` is freshly installed,
you'll need to run `rustup-init` to pull Cargo, the Rust toolchain, etc.

Tangentially to all of this, a `Dockerfile` is provided if preferred.

## Legal

This is released under the terms of the ISC License:

> Copyright 2022 The Wanderlust Group
> 
> Permission to use, copy, modify, and/or distribute this software for any
> purpose with or without fee is hereby granted, provided that the above
> copyright notice and this permission notice appear in all copies.
> 
> THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
> REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
> AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
> INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
> LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
> OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
> PERFORMANCE OF THIS SOFTWARE.
