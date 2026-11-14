#!/bin/sh
# The program of the state of the Redis service of the test run (SE-10.7, binding GN-13j), whose absolute path the test
# run gives the generated binding tests in SESSION_STORE_REDIS_STATE: it runs scripts/redis-state.mts with its
# arguments and its standard input on the node of the checkout (var/tools/bin/node), which make install-tools installs.
here=$(dirname "$0")
exec "$here/../var/tools/bin/node" "$here/redis-state.mts" "$@"
