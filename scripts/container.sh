#!/bin/sh
# The commands of the container program of the platform, which runs the Redis servers of the tests (SE-10.4).
#
#   scripts/container.sh program                      prints the name of the program
#   scripts/container.sh pull <image>                 pulls the image
#   scripts/container.sh present <image>              fails unless the image is installed
#   scripts/container.sh run <name> <argument> ...    runs a container <name> that is removed when it ends; the
#                                                     arguments are the options, the image and its command
#   scripts/container.sh delete <name> ...            stops and removes the containers
#   scripts/container.sh list <prefix>                prints the names of the containers that start with <prefix>-
#
# The platform names the program: Darwin runs Apple container and Linux runs docker. The image and its digest are one
# for both, so the tests of both platforms connect to the same bytes of the server. No variable selects the program;
# this file is the one place that holds the commands of both. A program that PATH does not hold fails every verb at
# once, with its name and what installs it.
#
# The address of a container has no command here: the command of a server writes `hostname -i` as its first line, on
# both programs, because a test reads the output of its container and the host reaches a container by that address.
set -u
usage() {
	echo "usage: scripts/container.sh program | pull <image> | present <image> | run <name> <argument> ... | delete <name> ... | list <prefix>" >&2
	exit 2
}
system=$(uname -s) || exit 1
case "$system" in
Darwin)
	program=container
	install='install Apple container (https://github.com/apple/container) and start it with `container system start`'
	;;
Linux)
	program=docker
	install='install Docker Engine (https://docs.docker.com/engine/install/) and let this user run it'
	;;
*)
	echo "scripts/container.sh: no container program is named for $system: Darwin runs Apple container and Linux runs docker" >&2
	exit 1
	;;
esac
[ "$#" -ge 1 ] || usage
verb=$1
shift
case "$verb:$#" in
program:0 | pull:1 | present:1 | list:1) ;;
program:* | pull:* | present:* | list:* | run:0 | delete:0) usage ;;
run:* | delete:*) ;;
*) usage ;;
esac
if ! command -v "$program" >/dev/null 2>&1; then
	echo "scripts/container.sh: $program is not found on PATH: $install" >&2
	exit 1
fi
case "$verb:$program" in
program:*)
	echo "$program"
	;;
pull:*)
	exec "$program" image pull "$1"
	;;
present:*)
	exec "$program" image inspect "$1" >/dev/null
	;;
run:container)
	name=$1
	shift
	exec container run --rm --progress none --name "$name" "$@"
	;;
run:docker)
	name=$1
	shift
	exec docker run --rm --name "$name" "$@"
	;;
delete:container)
	exec container delete --force "$@"
	;;
delete:docker)
	exec docker rm --force "$@"
	;;
list:container)
	names=$(container ls --all --quiet) || exit 1
	printf '%s\n' "$names" | { grep "^$1-" || :; }
	;;
list:docker)
	names=$(docker ps --all --format '{{.Names}}') || exit 1
	printf '%s\n' "$names" | { grep "^$1-" || :; }
	;;
esac
