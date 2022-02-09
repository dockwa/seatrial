-- uses https://tieske.github.io/date/, a pure-Lua date library
local date = require('date')

local ESOTERIC_FORMAT_REGEX = "^DAYS (%d+) SYEAR (%d+) EYEAR (%d+) SMON (%d+) EMON (%d+) SDAY (%d+) EDAY (%d+)$"

function generate_30_day_range()
	local today = date(true)
	local plus30 = today:copy():adddays(30)
	return {
		start_date = today:fmt('%F'),
		end_date = plus30:fmt('%F'),
	}
end

function is_valid_esoteric_format(arg)
	if arg.body_string:match(ESOTERIC_FORMAT_REGEX) == nil then
		return ValidationResult.Error("server responded with malformed body")
	end

	return ValidationResult.Ok()
end

return {
	generate_30_day_range = generate_30_day_range,
	is_valid_esoteric_format = is_valid_esoteric_format,
}
