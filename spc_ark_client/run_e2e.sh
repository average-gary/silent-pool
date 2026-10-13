#!/usr/bin/env bash
# Regtest e2e: stock captaind (unmodified bark, pinned to commit c419159) plus
# the modified miner client.
#
# Prerequisites:
#   * A local clone of https://gitlab.com/ark-bitcoin/bark checked out at
#     c419159, with a clean working tree. Point BARK_CHECKOUT at it, e.g.:
#         git clone https://gitlab.com/ark-bitcoin/bark /path/to/bark
#         git -C /path/to/bark checkout c419159
#         BARK_CHECKOUT=/path/to/bark ./run_e2e.sh
#   * bitcoind v31+; point BITCOIND_EXEC at the binary.
set -euo pipefail
cd "$(dirname "$0")"

BARK=${BARK_CHECKOUT:?set BARK_CHECKOUT to a clean bark checkout at c419159}
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$PWD/target}

# Stock server binary, built straight from the pinned checkout. Refuse to run
# against a modified tree so "stock server" stays honest.
(cd "$BARK" && git diff --quiet && git diff --cached --quiet) || { echo "bark checkout is modified" >&2; exit 1; }
# Separate target dir: sharing one with this workspace corrupts artifacts.
(cd "$BARK" && CARGO_TARGET_DIR=$CARGO_TARGET_DIR-captaind cargo build -p bark-server --bin captaind)

export CAPTAIND_EXEC=$CARGO_TARGET_DIR-captaind/debug/captaind
export BITCOIND_EXEC=${BITCOIND_EXEC:?set BITCOIND_EXEC to a bitcoind v31+ binary}
export TEST_DIRECTORY=${TEST_DIRECTORY:-$PWD/test}
export RUST_LOG=${RUST_LOG:-info}

cargo test --test e2e -- --nocapture "$@"
