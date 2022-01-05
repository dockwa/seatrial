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

## Legal

(c) 2022 The Wanderlust Group, All Rights Reserved

There's no major reason for this to remain proprietary given that it stands
independent of all other TWG codebases and uses no internal IP, but for the
time being, it's being developed in private while we flesh out the ideas. Some
sort of ISC/MPL-2.0/maybe even CC0-1.0 if we just don't care/etc. license could
be appropriate if we decide to release this to the public in any capacity.
