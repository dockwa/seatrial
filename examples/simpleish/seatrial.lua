-- uses https://tieske.github.io/date/, a pure-Lua date library
local date = require('date')

function generate_30_day_range()
	local today = date(true)
	local plus30 = today:copy():adddays(30)
	return {
		start_date = today:fmt('%F'),
		end_date = plus30:fmt('%F'),
	}
end

return {
	generate_30_day_range = generate_30_day_range,
}
