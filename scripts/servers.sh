#!/bin/sh
# The commands of the Redis servers of the tests (SE-10.4). bay starts and removes the servers as Compose projects, on
# Apple container on Darwin and on Docker on Linux; the engine of the platform pulls and finds the image, until bay has
# a command that pulls.
#
#   scripts/servers.sh check                          fails unless bay, the engine of the platform and the machine
#                                                     setup of bay are in place; changes nothing
#   scripts/servers.sh pull <image>                   pulls the image with the engine of the platform
#   scripts/servers.sh present <image>                fails unless the engine of the platform holds the image
#   scripts/servers.sh up <compose file>              starts the services of the project and returns after each accepts
#                                                     connections; a service that did not change is kept
#   scripts/servers.sh down <compose file>            removes the services of the project
#   scripts/servers.sh status                         prints the projects of bay as JSON, with the address of each
#                                                     service
#   scripts/servers.sh logs <compose file> <service>  prints the output of the service from its first line and follows
#                                                     it until the caller ends the command
#
# The tests, scripts/test-run.sh and make redis-image start this script alone. A program that PATH does not hold fails
# the verbs that need it, with its name and what installs it. No verb gives bay a terminal to ask a person on: the
# standard input of bay is empty.
set -u
usage() {
	echo "usage: scripts/servers.sh check | pull <image> | present <image> | up <compose file> | down <compose file> | status | logs <compose file> <service>" >&2
	exit 2
}
[ "$#" -ge 1 ] || usage
verb=$1
shift
case "$verb:$#" in
check:0 | status:0 | pull:1 | present:1 | up:1 | down:1 | logs:2) ;;
*) usage ;;
esac
# Fails unless PATH holds bay.
bay_found() {
	if ! command -v bay >/dev/null 2>&1; then
		echo 'scripts/servers.sh: bay is not found on PATH: install bay, which runs the Compose projects of the servers, and apply its machine setup with `bay install`' >&2
		exit 1
	fi
}
# Names the engine of the platform in $engine and fails unless PATH holds it.
engine_found() {
	system=$(uname -s) || exit 1
	case "$system" in
	Darwin)
		engine=container
		install='install Apple container (https://github.com/apple/container) and start it with `container system start`'
		;;
	Linux)
		engine=docker
		install='install Docker Engine (https://docs.docker.com/engine/install/) and let this user run it'
		;;
	*)
		echo "scripts/servers.sh: no engine is named for $system: Darwin runs Apple container and Linux runs docker" >&2
		exit 1
		;;
	esac
	if ! command -v "$engine" >/dev/null 2>&1; then
		echo "scripts/servers.sh: $engine is not found on PATH: $install" >&2
		exit 1
	fi
}
case "$verb" in
check)
	bay_found
	engine_found
	# bay up applies a part of the machine setup that is missing; a run must not, so it asks first.
	if ! report=$(bay check 2>&1 </dev/null); then
		echo 'scripts/servers.sh: the machine setup of bay is not in place: `bay install` applies it' >&2
		printf '%s\n' "$report" >&2
		exit 1
	fi
	;;
pull)
	engine_found
	exec "$engine" image pull "$1"
	;;
present)
	engine_found
	exec "$engine" image inspect "$1" >/dev/null
	;;
up)
	bay_found
	exec bay -f "$1" up </dev/null
	;;
down)
	bay_found
	exec bay -f "$1" down </dev/null
	;;
status)
	bay_found
	exec bay status --json </dev/null
	;;
logs)
	bay_found
	exec bay -f "$1" logs -f "$2" </dev/null
	;;
esac
