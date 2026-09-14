#!/bin/sh
# Local foundation gate; does not claim acceptance of unimplemented packages.
set -eu
TASK_VALIDATION_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$TASK_VALIDATION_ROOT"
node scripts/check-architecture.mjs
node --test scripts/check-architecture.test.mjs scripts/preflight.test.mjs scripts/source-fingerprint.test.mjs
node --test fixtures/orders/test/*.test.mjs
cargo test --workspace --locked --offline
