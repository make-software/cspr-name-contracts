#!/usr/bin/env bash
# Upgrades the deployed CSPR.name contracts to the "dapp registry" version and
# registers them as members of the dapp, with the NameToken as the registry.
#
# Run it once, as the account that installed the contracts, with the livenet
# configuration of the target network selected (for example
# `ODRA_CASPER_LIVENET_ENV=nctl`). Every upgrade sends a transaction, so a
# rerun upgrades again; the final registration step is idempotent.
#
# Usage:
#   scripts/upgrade-dapp-registry.sh [--name ..] [--description ..] [--website_url ..] [--icon_url ..]
#
# The options are passed to the `register-dapp` scenario.
#
# Environment:
#   SKIP_BUILD=1     reuse the wasm files already in ./wasm
#   FRESH_INSTALL=1  the packages were installed with Odra 2.3 or newer (for
#                    example a fresh nctl deployment), so they already have an
#                    upgrade group and use the default package named keys.
#                    Without it the flags match the public deployment, whose
#                    packages were installed with an older Odra.
#   GAS              gas per upgrade, default 400cspr
set -euo pipefail
cd "$(dirname "$0")/.."

GAS="${GAS:-400cspr}"

cli() {
  cargo run -q -p casper-name-cli -- "$@"
}

upgrade() {
  local contract="$1"; shift
  echo "==> upgrade $contract $*"
  cli upgrade "$contract" --gas "$GAS" "$@"
}

if [ "${SKIP_BUILD:-0}" != "1" ]; then
  echo "==> Building wasm files"
  cargo odra build
fi

# Plain strings, expanded unquoted below: macOS ships bash 3.2, where an empty
# array trips `set -u`.
if [ "${FRESH_INSTALL:-0}" = "1" ]; then
  GROUP=""
  REVERSE_RESOLVER_KEY=""
else
  # Packages installed with Odra 2.2 or earlier get their upgrade group created
  # on this first upgrade. The NameToken was upgraded before and has one.
  GROUP="--force-create-upgrade-group"
  # The ReverseResolver package hash lives under a legacy named key.
  REVERSE_RESOLVER_KEY="--package-named-key ReverseResolver_contract_package"
fi

# The order matters: the Controller reads the registry from the Registrar
# during its upgrade, and `register-dapp` calls `get_dapp_registry` on every
# member, which only the upgraded code exposes.
# shellcheck disable=SC2086
upgrade NameToken
upgrade Registrar $GROUP
upgrade Controller $GROUP
upgrade DefaultResolver $GROUP
upgrade ReverseResolver $GROUP $REVERSE_RESOLVER_KEY

echo "==> scenario register-dapp"
cli scenario register-dapp "$@"

echo "==> Done. Verify with scripts/verify-dapp-registry.sh"
