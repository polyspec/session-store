#!/bin/sh
# Runs the tests of `make test` as a run that owns its servers and its temporary directory (SE-10.4).
#
#   scripts/test-run.sh <command> [<argument> ...]
#
# The command runs in a process group of its own with TMPDIR, a directory of this run, and SESSION_STORE_PROJECTS, the
# directory below it in which each test writes the Compose file of its servers, <project>/compose.yaml, before it
# starts them. The directory is named after the process of the run, session-store-test-<pid>, and a project is named
# after it, so no two runs of a machine hold one name. When the command ends or fails, and when the run receives
# SIGINT or SIGTERM, which it passes on to the process group of the command, the script waits for the command, runs
# down for every Compose file that the directory still holds and removes the directory of the run. A project that
# down does not remove fails the run, which then keeps the directory and names the file.
#
# scripts/servers.sh holds the commands. When bay or a part of the machine setup that a project needs is missing, the
# run fails before the command starts, with what is missing and what gives it.
set -u
if [ "$#" -lt 1 ]; then
	echo "usage: scripts/test-run.sh <command> [<argument> ...]" >&2
	exit 2
fi
servers=$(dirname "$0")/servers.sh
"$servers" check || exit 1
directory=$(mktemp -d) || exit 1
projects=$directory/session-store-test-$$
mkdir "$projects" || exit 1
# Removes the servers and the directory of the run; fails when a project is not removed.
remove() {
	kept=0
	for file in "$projects"/*/compose.yaml; do
		[ -f "$file" ] || continue
		if ! report=$("$servers" down "$file" 2>&1); then
			echo "scripts/test-run.sh: the servers of $file are not removed; the directory of the run is kept:" >&2
			printf '%s\n' "$report" >&2
			kept=1
		fi
	done
	[ "$kept" -eq 0 ] || return 1
	rm -rf "$directory"
}
# Job control gives the command a process group of its own, so that a signal reaches every process of the command.
set -m
SESSION_STORE_PROJECTS=$projects TMPDIR=$directory "$@" &
command=$!
stop() {
	kill "-$1" "-$command" 2>/dev/null
	wait "$command"
	remove
	exit "$2"
}
trap 'stop INT 130' INT
trap 'stop TERM 143' TERM
wait "$command"
status=$?
if ! remove && [ "$status" -eq 0 ]; then
	status=1
fi
exit "$status"
