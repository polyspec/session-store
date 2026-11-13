#!/bin/sh
# Runs the tests of `make test` as a run that owns its containers and its temporary directory (SE-10.4).
#
#   scripts/test-run.sh <container> <command> [<argument> ...]
#
# The command runs in a process group of its own with SESSION_STORE_CONTAINERS, the prefix of the names of the
# containers of this run, and TMPDIR, a directory of this run. When the command ends or fails, and when the run receives
# SIGINT or SIGTERM, which it passes on to the process group of the command, the script waits for the command, deletes
# every container whose name starts with the prefix and removes the directory. <container> is the path of Apple
# container; when it is empty, the run has no container to delete.
set -u
if [ "$#" -lt 2 ]; then
	echo "usage: scripts/test-run.sh <container> <command> [<argument> ...]" >&2
	exit 2
fi
container=$1
shift
prefix=session-store-test-$$
directory=$(mktemp -d) || exit 1
remove() {
	if [ -n "$container" ]; then
		names=$("$container" ls --all --quiet | grep "^$prefix-")
		if [ -n "$names" ]; then
			# shellcheck disable=SC2086 # one name per word
			"$container" delete --force $names >/dev/null || echo "the containers of $prefix are not deleted" >&2
		fi
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
