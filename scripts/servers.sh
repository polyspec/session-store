#!/bin/sh
# The commands of the Redis servers of the tests (SE-10.4). bay starts and removes the servers as Compose projects, on
# Apple container on Darwin and on Docker on Linux, gives their names and addresses and pulls and finds the image; this
# file holds no command of an engine.
#
#   scripts/servers.sh check                          prints the version of bay and fails unless the machine setup
#                                                     that a project needs is in place; changes nothing
#   scripts/servers.sh pull <image>                   pulls the image and fails unless the store then holds it
#   scripts/servers.sh plan <compose file>            prints as JSON the name and the certificate paths of each service,
#                                                     as up will create them; changes nothing
#   scripts/servers.sh up <compose file>              starts the services of the project, returns after each accepts
#                                                     connections and prints their addresses as JSON; it pulls no
#                                                     image and fails before it creates anything when one is missing
#   scripts/servers.sh images <compose file>          fails unless the images of the project are installed
#   scripts/servers.sh exec <compose file> <service> <command> ...
#                                                     runs the command in the container of the service and ends with
#                                                     its status
#   scripts/servers.sh logs <compose file> <service>  prints the output of the service from its first line and follows
#                                                     it until the service or the caller ends
#   scripts/servers.sh output <compose file> <service>
#                                                     prints the output of the service so far and ends
#   scripts/servers.sh down <compose file>            removes the services of the project
#
# bay names a project after the directory of its Compose file. The tests, scripts/test-run.mts and make redis-image
# start this script alone, and it alone starts bay, by the words of BAY: bay of PATH when BAY is not set, and for
# example `sudo -n <path>/bay` where every command of bay runs as root. A program of BAY that PATH does not hold fails
# every verb, with its name and what installs bay. No verb gives bay a terminal to ask a person on: its standard input
# is empty.
set -u
usage() {
	echo "usage: scripts/servers.sh check | pull <image> | plan <compose file> | up <compose file> | images <compose file> | exec <compose file> <service> <command> ... | logs <compose file> <service> | output <compose file> <service> | down <compose file>" >&2
	exit 2
}
[ "$#" -ge 1 ] || usage
verb=$1
shift
case "$verb:$#" in
check:0 | pull:1 | plan:1 | up:1 | images:1 | down:1 | logs:2 | output:2) ;;
exec:0 | exec:1 | exec:2) usage ;;
exec:*) ;;
*) usage ;;
esac
# BAY names the words that start bay, separated by spaces; bay of PATH when it is not set.
bay=${BAY:-bay}
program=${bay%% *}
if ! command -v "$program" >/dev/null 2>&1; then
	echo "scripts/servers.sh: $program is not found on PATH: install bay, which runs the Compose projects of the servers (https://github.com/polyspec/bay), and apply its machine setup with \`bay install\`; BAY names the words that start bay" >&2
	exit 1
fi
case "$verb" in
check)
	# The first line of a run: the bay that starts its servers. A run compares no version; the harness fails a
	# document of bay of another contract than the one that it reads.
	# shellcheck disable=SC2086 # BAY is words
	if ! $bay version </dev/null; then
		echo 'scripts/servers.sh: bay gives no version: install a release of bay that has the command version' >&2
		exit 1
	fi
	# up names a missing part of the setup too; this check gives a run one failure before its build.
	# shellcheck disable=SC2086 # BAY is words
	if ! report=$($bay check -scope run 2>&1 </dev/null); then
		echo 'scripts/servers.sh: the machine setup of bay is not in place: `bay install` applies it' >&2
		printf '%s\n' "$report" >&2
		exit 1
	fi
	;;
pull)
	# bay pulls the images of a project, so the image is the one service of a project that is written for the pull.
	# bay names the project after the directory and accepts letters, digits, - and _ alone; mktemp without a template
	# gives a name with a dot.
	directory=$(mktemp -d "${TMPDIR:-/tmp}/session-store-pull-XXXXXXXX") || exit 1
	printf 'services:\n  redis:\n    image: "%s"\n' "$1" >"$directory/compose.yaml" || exit 1
	# shellcheck disable=SC2086 # BAY is words
	$bay -f "$directory/compose.yaml" pull </dev/null
	status=$?
	# A pull that ends with status 0 is no evidence that the store holds the image, so the store is asked for it, and a
	# pull after which the store does not hold it fails with the image.
	if [ "$status" -eq 0 ]; then
		# shellcheck disable=SC2086 # BAY is words
		if ! $bay -f "$directory/compose.yaml" images </dev/null; then
			echo "scripts/servers.sh: bay pull ended, but the local store does not hold $1" >&2
			status=1
		fi
	fi
	rm -rf "$directory"
	exit "$status"
	;;
plan)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" plan --json </dev/null
	;;
up)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" up -no-pull --json </dev/null
	;;
images)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" images </dev/null
	;;
exec)
	file=$1
	shift
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$file" exec "$@" </dev/null
	;;
logs)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" logs -f "$2" </dev/null
	;;
output)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" logs "$2" </dev/null
	;;
down)
	# shellcheck disable=SC2086 # BAY is words
	exec $bay -f "$1" down </dev/null
	;;
esac
