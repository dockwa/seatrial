seatrial(1) "https://engineering.dockwa.com" "situational-mock-based load testing"

# SYNOPSIS

*seatrial* <base_url> <req_situation> [<situations...>] [-m <multiplier>]

# DESCRIPTION

*seatrial* is a load generation tool for HTTP/1.1 services built to simulate
traffic on known-common flows, particularly in monolith-ish applications. It
operates under the model of: one or more Situations are executed in parallel,
involving one or more Grunts who will go through the flows of using the backing
service(s) based on the rules defined in their associated Persona. This makes
*seatrial* a decent fit for testing otherwise constant-traffic applications'
behaviors under fairly-predictably-bursty load.

This tool is technically capable of performing tests like "just attack this
endpoint until it falls over", but is not designed around them. It's also not
(yet?) well-suited to unpredictable loads - it can probably be used in a
fuzzing manner to discover such breaking points, but `seatrial` is currently
optimized for taking historical learnings plus data gleamed from the rest of
your observability stack, and preventing repeats of the same outages (and
indeed, for helping developers make such scale events, Non-Events).

# ARGUMENTS

_base\_url_ is the URL to which all paths in the provided Situations will be
appended. For example, to load-test a Rails application at
http://localhost:8000, _base\_url_ would be http://localhost:8000, and paths in
the Situation might be, for example, /admin.

_req\_situation_ is the first Situation file that should be executed, in
*seatrial(5)* format. This one is required.

_situations_ are one or more further Situation files, also in *seatrial(5)*
format, that should be executed in parallel to the _req\_situation_.

_multiplier_ is an integer greater than or equal to 1 (the default), to linearly
scale the load applied to the application. This interacts with _Grunt.count_ in
*seatrial(5)*, which defines the number of "users" to simulate for a given
Persona.

# IMPLEMENTATION NOTES

## THREADING AND RESOURCE UTILIZATION

*seatrial* makes use of numerous threads, and currently isolates almost
everything into a separate thread, primarily for implementation simplicity. This
means that your system provides the upper limit for how much work *seatrial* can
do, though it should be fairly difficult to reach that ceiling before maxing out
some other resource (likely your network). Threads are spawned for each of the
following:

- 1x main controller thread which mostly sits waiting for situation completions
- 1x/situation situation controller thread, which mostly sits waiting for grunt
  completions
- 1x/grunt/situation activity thread, which controls all Lua and HTTP(S)
  activity for the given grunt, and shares no memory with other activity
  threads.

Thus, given five provided situation files, each spinning up 2 grunts, each at
_count: 2_, we end up with 1+5+(5x2x2)=26 system threads. If we pass _-m 3_ at
the command line to multiply the load by three, we end up with 1+5+(5x2x2x3)=66
system threads.

## DISTRIBUTED TESTING

*seatrial* currently lacks distributed testing abilities, and at time of
writing, such functionality is not yet concretely planned.

# SEE ALSO

- *seatrial(5)*
- *seatrial.lua(3)*

# AUTHORS

Built by Dockwa Engineering. Sources can be found at
https://github.com/dockwa/seatrial.
