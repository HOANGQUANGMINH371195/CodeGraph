#!/bin/sh
# Required Linux sandbox fixture gate; missing capability is failure.
set -eu
TASK_SANDBOX_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$TASK_SANDBOX_ROOT"
if [ "$(uname -s)" != Linux ]; then
    echo "required sandbox gate supports Linux only" >&2
    exit 1
fi
export GRAPH_REQUIRE_SANDBOX=1
scripts/with-local-tools cargo test -p graph-execution --test sandbox --locked --offline -- --nocapture
scripts/with-local-tools cargo test -p graph-execution --test rpc_fixture --locked --offline -- --nocapture
