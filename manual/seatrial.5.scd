seatrial(5) "https://engineering.dockwa.com" "situational-mock-based load testing"

# SYNOPSIS

*seatrial(1)* is configured with one declarative, plaintext, RON-formatted file
per Situation to be tested. This manual page provides a quick summary of RON
(Rusty Object Notation - further reading in _SEE ALSO_ below) syntax and a
walkthrough of the fields that make up a *seatrial* Situation config.

# RON SYNTAX

- _structs_ are denoted by an optional name followed by parentheses, and contain
  positional or named arguments
- _tuples_ look almost identical to _structs_, but have no preceeding name, and
  cannot contain named arguments
- _maps_ are denoted by curly braces, and contain key-value pairs, where the
  keys are _strings_ (see below) and the values are of whatever type is
  appropriate in the context
- _arrays_ are denoted by brackets; the values within are of whatever type is
  appropriate in the context
- _strings_ are double-quoted
- _booleans_ are one of the following literal values:
	- true
	- false
- _numbers_ follow standard integer and float formatting rules as one would
  expect, plus support for 0x and 0b prefixes, for hex and binary numbers,
  respectively

# SEATRIAL CONFIGURATION FORMAT

*seatrial* configurations are an unnamed struct, and thus, the entire config
will be wrapped in parentheses (this does mean all useful data is indented a
level if formatted in the typical RON style seen in the wild, but the same is
largely true of JSON anyway).

## lua_file

While optional in the data model, _lua\_file_ is currently a
functionally-required key in the configuration, containing a string path, relative to
the situation file, where the Lua user script (perhaps using *seatrial.lua(3)*
APIs) can be found. This file, when executed in a Lua interpreter, must return a
table with string keys and function values to be used elsewhere in the pipeline.
More on Lua interactions later. For an example of a _lua\_file_, see the
_examples/_ directory in the source tree.

## grunts

_grunts_ is an array of Grunts, *seatrial*'s tongue-in-cheek name for simulated
users. Grunts are simple creatures: they follow the rules provided in their
_persona_ (a _Persona_ struct as described below), and additionally have a
_base\_name_ (a string) and a _count_ (an integer), which together with the
global _multiplier_ (see *seatrial(1)*), determines how many of this Grunt
should be created.

## Persona

_personas_ describe the actions a Grunt will take, and a few other
characteristics regarding how HTTP requests will be made. They are defined in an
anonymous struct as follows:

- _timeout_ is one of the following enum members, and describes the *overall*
  timeout that will be applied to HTTP requests within the Persona:
	- _Seconds(<integer>)_
	- _Milliseconds(<integer>)
- _headers_ is a map of strings to _References_, described below
- _sequence_ is an array of _Actions_, described below

## Persona: References

_References_ are found as the type of numerous _Action_ arguments, as well as
_Persona.headers_ described above. They are enums, and take one of the following
forms:

- _Value(<string>)_ hard-codes a string value, and is often useful for query
  parameters, headers, etc. that can be known *statically* (meaning no Lua is
  needed to calculate their value). Hard-coded string _Values_ will *always* be
  more performant than anything that needs to cross the *seatrial*-Lua boundary,
  and thus should be used whenever possible.

- _LuaValue_ plucks the Lua value returned by the last step in the pipeline,
  stringifies it, and passes that stringified value to the caller. This is a
  fairly niche state, useful when only one parameter/header/etc. in an HTTP
  request needs to be dynamic. If the data in the pipe is not a Lua value, doesn't
  exist to begin with, or is not stringifiable (notably, functions, tables, and
  nil will never be stringified), a fatal error will be thrown and the Grunt will
  stop execution.

- _LuaTableIndex(<integer>)_ and _LuaTableValue(<string>)_ pluck a value from
  the Lua table returned by the last step in the pipeline by either its numeric
  index or its string key, as appropriate. This is almost always the most useful
  way to plumb dynamic data to an HTTP request. If the data in the pipe is not a
  Lua table, doesn't exist to begin with, if the specified key doesn't exist in
  the table, or if the key resolves to a value that cannot be stringified
  (notably, functions, tables, and nil will never be stringified), a fatal error
  will be thrown and the Grunt will stop execution.

## Persona: Actions

_Actions_ are the proverbial meat and potatoes of *seatrial*, although
properly-seasoned tofu and potatoes also makes for an excellent meal and is
recommended. Each Action in a Sequence *may, optionally*, populate data in the
pipeline for the next step to read. The steps that do so are documented as such
below. Actions are named enum members in RON syntax terms, though some are
double-enumed (think of this as namespacing, perhaps), which is an
implementation detail that may change in a future version (deprecation notices
will be provided).

- _ControlFlow(<action>)_ is a namespace containing only one action:
	- _ControlFlow(GoTo(index: <integer>, max_times: <optional integer>))_ jumps
	  to the specified index in the pipeline, presuming it exists, allowing for
	  looping and/or skipping of steps. If _max\_times_ is specified, it serves as
	  an end to the loop after that number of arrivals at _GoTo_

- _Http(<action>)_ is a namespace containing the following actions:
	- _Http(Delete(<args>))_
	- _Http(Get(<args>))_
	- _Http(Head(<args>))_
	- _Http(Post(<args>))_
	- _Http(Put(<args>))_

	Each of these take the same _args_, of which _url_ is required, and the rest
	are all optional:

		- _url_ is a string containing the relative (to the _base\_url_ provided
		  at the CLI; see *seatrial(1)*) path to send the request to

		- _body_ is a _Reference_ containing the request body. Currently
		  non-string bodies are relatively untested, and thus not fully defined,
		  behavior (this is a known issue in *seatrial*)

		- _headers_ is a map of strings to _References_ containing HTTP headers.
		  These are merged with (and in the event of a collision, will override)
		  the _headers_ of the Persona.

		- _params_ is a map of strings to _References_ containing HTTP query
		  parameters (which are ultimately passed as part of the URL).

		- _timeout_ follows the same rules as _Persona.timeout_ and will
		  override the Persona-provided timeout for this request.

	A successful request with *any* status code (not just a 2xx) will be placed
	in the pipe for the next step to read (for details on how to access this
	from a Lua function, see _LuaFunction_ below). Failures (perhaps due to
	timeout, or due to some other system-level failure, like a socket issue)
	will immediately end Sequence and Grunt execution, and terminate the thread.

- _LuaFunction(<string>)_ runs the specified Lua function, and places the return
  value on the stack. While its implementation in *seatrial* is consistent, it's
  also context-specific, so both usecases are documented below. If the Lua
  function raises an exception or cannot be found, Grunt execution stops
  immediately.

	If the data in the pipe is an HTTP request (either by way of this
	_LuaFunction_ call being the sole step following an Http(\*) step, or as
	part of a _Combinator_, described below), the function will be called with a
	table as the sole argument, containing the following properties. This
	usecase is almost exclusively useful for _Validators_; the requirements of
	_LuaFunction_ in a _Validator_ context is described in _Validator_ below.

		- _body_, a table of 8-bit integers representing the raw bytes of the
		  response body. This will always exist, albeit potentially with a table
		  length of 0.

		- _body\_string_, which will be _nil_ if the body was not parseable as a
		  UTF-8 string (no other encodings are supported; it's 2022 at time of
		  writing, use UTF-8 or provide a Lua library to handle other
		  encodings), or will be the UTF-8 string the body provided, with no
		  further processing. If the body is, say, JSON, this is the field that
		  is likely most useful to libraries like _json.lua_ (see _SEE ALSO_
		  below).

		- _content\_type_, a string which contains the Content-Type as dictated
		  by the returned headers

		- _headers_, a table of strings to strings that is unmodified from
		  whatever the server returned in the response headers

		- _status\_code_, a 16-bit integer containing the status code as
		  provided by the server

	If there is no data, or any other type of data, in the pipe, consider the
	argument, if any (depending on the version of *seatrial* you have) passed to
	the function to be undefined behavior, unstable, and unusable. In this case,
	the _LuaFunction_ is likely being used to generate values for future steps
	in the Sequence, and should be (within Lua, at least) self-sufficient.

- _Combinator(Combinator([Validator, ...]))_ is a namespace containing three
  actions, which allow running multiple _Validators_ (see below) over the same
  pipe data:

	- _AllOf([Validator, ...])_ runs each listed validator in sequence and
	  requires that all of them return with an _Ok_ or _OkWithWarnings_
	  status. If any return with an _Error_ status, execution stops immediately
	  for the entire persona's pipeline.

	- _AnyOf([Validator, ...])_ runs each listed validator in sequence and
	  requires that one of them return with an _Ok_ or _OkWithWarnings_ status.
	  This combinator ends once it finds such a status, or fails the entire
	  pipeline and persona if all listed validators return _Error_.

	- _NoneOf([Validator, ...])_ runs each listed validator in sequence and
	  expects that all of them return an _Error_ status. Immediately upon finding
	  an _Ok_ or _OkWithWarnings_ status, the pipeline is failed.


- _Validator(<validator>)_ (namespace prefix not allowed in a combinator array;
  this is an implementation detail that bleeds through to the UX, sorry) allows
  validation of data in the pipe; currently all _Validators_ work on HTTP
  responses only. In general, they have one of two prefixes: _WarnUnless_, which
  return _Ok_ or _OkWithWarnings_, and _Assert_, which return _Ok_ or immediately
  fail the pipeline on _Error_ (barring interactions with _Combinators_, see
  above). Further, _LuaFunction_ is a valid _Validator_; functions receive the
  data in the pipe as their first argument (see _LuaFunction_ above) and must
  return a _ValidationResult_ object as documented in *seatrial.lua(3)*. An
  example _LuaFunction_ validator is provided in
  _examples/simpleish/seatrial.lua_ in the *seatrial* source tree.

	All of the following have _Assert..._ and _WarnUnless..._ forms, so only the
	shared parts of their names are listed here for brevity. For example,
	_HeaderEquals_ should be expanded to _AssertHeaderEquals_ when writing config
	files.

	- _HeaderEquals(<string>, <string>)_ takes a case-insensitive header
	  name and case-sensitive expected value

	- _HeaderExists(<string>_ takes a case-insensitive header name, and simply
	  checks whether it is present in the response at all

	- _StatusCode(<u16>)_ takes a 16-bit unsigned integer and checks whether the
	  HTTP status code exactly matches

	- _StatusCodeInRange(<u16>, <u16>)_ takes two 16-bit unsigned integers and
	  checks whether the HTTP status code is greater than or equal two the
	  first, *and* is less than or equal to the second

# EXAMPLE

Further examples are provided in the _examples/_ directory in the source tree.

```
(
    lua_file: "seatrial.lua",

    grunts: [
        (
            base_name: "Postmaster General",
            count: 1,
            persona: (
                timeout: Seconds(30),
                sequence: [
                    LuaFunction("generate_profile"),
                    Http(Post(
                        url: "/profile",
                        body: LuaTableValue("profile"),
                        headers: { "Content-Type": Value("application/json") },
                    )),
                    Combinator(AllOf([
                        WarnUnlessStatusCodeInRange(200, 299),
                        WarnUnlessHeaderExists("X-Never-Gonna-Give-You-Up"),
                    ]))
                ]
            ),
        ),
        (
            base_name: "Reloader Grunt",
            count: 10,
            persona: (
                timeout: Seconds(30),

                sequence: [
                    LuaFunction("generate_30_day_range"),
                    Http(Get(
                        url: "/calendar",
                        params: {
                            "start_date": LuaTableValue("start_date"),
                            "end_date": LuaTableValue("end_date"),
                        },
                    )),
                    Combinator(AllOf([
                        WarnUnlessStatusCodeInRange(200, 299),
                        WarnUnlessHeaderExists("X-Never-Gonna-Give-You-Up"),
                        LuaFunction("is_valid_esoteric_format"),
                    ])),
                    ControlFlow(GoTo(index: 0, max_times: 2)),
                ],
            ),
        ),
    ],
)
```

# SEE ALSO

- *seatrial(1)*
- *seatrial.lua(3)*
- https://github.com/ron-rs/ron
- https://github.com/rxi/json.lua

# AUTHORS

Built by Dockwa Engineering. Sources can be found at
https://github.com/dockwa/seatrial.
