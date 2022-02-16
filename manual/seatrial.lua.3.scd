seatrial.lua(3) "https://engineering.dockwa.com" "situational-mock-based load testing"

# SYNOPSIS

*seatrial* user-facing Lua API for dynamic pipeline values

# DESCRIPTION

*seatrial(1)* is generally configured in a declarative style in RON format, as
described in *seatrial(5)*. However, sometimes it's useful to generate values
dynamically at runtime (perhaps, today's date), validate responses in some way
*seatrial* doesn't provide first-class instructions for (perhaps most notably,
anything involving request bodies currently requires Lua instrumentation), or
perhaps even a combination thereof (generating a session token in sequence step
1 which is then referred to in the remainder of the grunt's lifetime, preserved
in a Lua global variable across sequence loops). The semantics of how to refer
to Lua files and values generated therein are described in *seatrial(5)*; this
manual page describes the "standard library", or perhaps more accurately,
utility belt, such Lua code has access to, above and beyond the Lua standard
library itself.

Each top-level value is accessible by their bare name in Lua code as global
variables - in other words, the _ValidationResult_ section describes a Lua table
accessible as _ValidationResult_ directly, and its members, for example,
_ValidationResult.Ok()_.

# ValidationResult

_ValidationResult_ is a fairly-direct Lua mapping of a Rust enum by the same
name, having three function members: _Ok_, _OkWithWarnings_, and _Error_. A
table returned by one of these three functions is the *only* allowed response
from a _LuaFunction_ validator, notably, *nil (as is the default return value
from a Lua function) is never allowed as a validator return value*.

All _ValidationResult_ members currently empty the pipe, leaving nothing for the
next step to read. This may change in a future version of *seatrial*.

## ValidationResult.Ok()

Used to denote that a validation succeeded, and will allow the pipeline to
proceed to the next step.

```
function noop_validator()
	return ValidationResult.Ok()
end
```

## ValidationResult.OkWithWarnings(warnings)

Accepts a list (table) of warning strings. Used to denote that a validation
succeeded, but non-fatal warnings were emitted. This still allows the pipeline
to proceed to the next step, but the warnings will be logged.

```
function warn_validator(response)
	if response.body_string:match("stars") == nil then
		return ValidationResult.OkWithWarnings({ "expected ratings API to discuss stars" })
	end

	return ValidationResult.Ok()
end
```

## ValidationResult.Error(err)

Accepts an error message as a string. Used to denote that a validation fatally
failed, and that the pipeline should stop here (unless otherwise overridden by a
_Combinator_, see *seatrial(5)*).

```
function err_validator(response)
	if response.body_string:match("stars") == nil then
		return ValidationResult.Error("expected ratings API to discuss stars")
	end

	return ValidationResult.Ok()
end
```

# IMPLEMENTATION NOTES

## LIFECYCLES AND LIFETIMES

The Lua VM for a Grunt is initialized exactly once, when the Grunt's thread is
started. Thus, global variables (be them seatrial-borne, such as with the API
described in this manual, or user-generated data from the pipeline stored in a
global) live until the end of the pipeline and thus thread, or until garbage
collected by the usual Lua means. This makes Lua globals an
untested-but-theoretically-sane place to stash dynamic data that may be
necessary in later pipeline runs (perhaps a session token returned by an
authentication API). First class support for non-pipe-aligned data flows
(perhaps a simple key-value store) is being considered, but has no ETA.

# SEE ALSO

- *seatrial(1)*
- *seatrial(5)*

# AUTHORS

Built by Dockwa Engineering. Sources can be found at
https://github.com/dockwa/seatrial.
