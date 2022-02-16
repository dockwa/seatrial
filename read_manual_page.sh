#!/bin/sh

set -eu

# read_manual_page.sh colloquial_name(section)
#
# ex: read_manual_page.sh seatrial.lua(3)

die() {
	message="${1:-died without an error message, this is a bug in the script}"
	code="${2:-1}"

	echo "${message}" >&2
	exit "${code}"
}

hash scdoc 2>/dev/null || die "rendering manual pages requires scdoc to be installed (https://git.sr.ht/~sircmpwn/scdoc)"
hash man 2>/dev/null || die "rendering manual pages requires man(1) to be in your PATH, perhaps install man, mandoc, or similar"

# given "seatrial.lua(3)", return "manual/seatrial.lua.3.scd"
source_path_from_user_input() {
	echo "${1}" | awk -F"[()]" '{print "manual/" $1  "."  $2 ".scd"}'
}

MANUAL_PAGE_SOURCE=$(source_path_from_user_input "${1}")
MANUAL_PAGE_RENDERED=$(echo "${MANUAL_PAGE_SOURCE}" | sed "s#\.scd\$##")
scdoc < "${MANUAL_PAGE_SOURCE}" > "${MANUAL_PAGE_RENDERED}"|| die "failed to render manual page ${MANUAL_PAGE_SOURCE}"
exec man "${MANUAL_PAGE_RENDERED}"
