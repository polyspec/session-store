#!/bin/sh
# Runs the tests of `make test` as a run that owns its containers and its temporary directory (SE-10.4).
#
#   scripts/test-run.sh <command> [<argument> ...]
#
# The command runs in a process group of its own with SESSION_STORE_CONTAINERS, the prefix of the names of the
# containers of this run, and TMPDIR, a directory of this run. When the command ends or fails, and when the run receives
# SIGINT or SIGTERM, which it passes on to the process group of the command, the script waits for the command, deletes
# every container whose name starts with the prefix and removes the directory. The container program is the one of the
# platform, whose commands scripts/container.sh holds; when the program is not found, the run fails before the command
# starts, with the name of the program and what installs it.
set -u
if [ "$#" -lt 1 ]; then
	echo "usage: scripts/test-run.sh <command> [<argument> ...]" >&2
	exit 2
fi
container=$(dirname "$0")/container.sh
"$container" program >/dev/null || exit 1
prefix=session-store-test-$$
directory=$(mktemp -d) || exit 1
remove() {
	if names=$("$container" list "$prefix"); then
		if [ -n "$names" ]; then
			# shellcheck disable=SC2086 # one name per word
			"$container" delete $names >/dev/null || echo "the containers of $prefix are not deleted" >&2
		fi
	else
		echo "the containers of $prefix are not listed" >&2
	fi
	rm -rf "$directory"
}
# Job control gives the command a process group of its own, so that a signal reaches every process of the command.
set -m
SESSION_STORE_CONTAINERS=$prefix TMPDIR=$directory "$@" &
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
remove
exit "$status"
