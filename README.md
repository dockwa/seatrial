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

## Legal

(c) 2022 The Wanderlust Group, All Rights Reserved

There's no major reason for this to remain proprietary given that it stands
independent of all other TWG codebases and uses no internal IP, but for the
time being, it's being developed in private while we flesh out the ideas. Some
sort of ISC/MPL-2.0/maybe even CC0-1.0 if we just don't care/etc. license could
be appropriate if we decide to release this to the public in any capacity.
