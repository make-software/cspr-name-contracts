#!/usr/bin/env bash
# Verifies that the deployed CSPR.name contracts run the "dapp registry"
# version and are registered in the dapp, with the NameToken as the registry.
#
# Reads only. Exits non-zero on the first failed check. Select the network
# with the usual livenet configuration (for example
# `ODRA_CASPER_LIVENET_ENV=nctl`).
set -euo pipefail
cd "$(dirname "$0")/.."

cli() {
  cargo run -q -p casper-name-cli -- --json "$@"
}

# Prints the `result` field of a `contract` call, or the error text.
call() {
  cli contract "$@" 2>&1 | python3 -c '
import json, sys
raw = sys.stdin.read()
# The JSON document starts on its own line; log lines may precede it.
lines = raw.splitlines()
try:
    start = lines.index("{")
    print(json.loads("\n".join(lines[start:]))["result"])
except (ValueError, KeyError):
    print("ERROR: " + (lines[-1].strip() if lines else "no output"))
'
}

# Package hash (64 hex chars) of a deployed contract, from the contracts file.
address_of() {
  cli status | python3 -c '
import json, sys
name = sys.argv[1]
doc = json.load(sys.stdin)
for c in doc["registered"]:
    if c["ident"] == name and c["deployed"]:
        print(c["address"].removeprefix("hash-"))
        break
' "$1"
}

ok() { echo "  [ok]   $1"; }
fail() { echo "  [FAIL] $1"; exit 1; }

echo "==> Deployed contracts"
NAME_TOKEN=$(address_of NameToken)
[ -n "$NAME_TOKEN" ] || fail "NameToken is not in the contracts file"
echo "  NameToken: hash-$NAME_TOKEN"

echo "==> NameToken is the registry"
registry=$(call NameToken get_dapp_registry)
[[ "$registry" == *"$NAME_TOKEN"* ]] || fail "NameToken.get_dapp_registry returned: $registry"
ok "get_dapp_registry returns the NameToken itself"

contracts=$(call NameToken get_dapp_contracts)
[[ "$contracts" == "[Key::Hash($NAME_TOKEN)"* ]] || fail "NameToken.get_dapp_contracts does not start with the NameToken: $contracts"
ok "get_dapp_contracts lists the NameToken first"

echo "==> Members"
for member in DefaultResolver Registrar Controller ReverseResolver; do
  address=$(address_of "$member")
  if [ -z "$address" ]; then
    echo "  [skip] $member is not deployed"
    continue
  fi
  registry=$(call "$member" get_dapp_registry)
  [[ "$registry" == *"$NAME_TOKEN"* ]] || fail "$member.get_dapp_registry returned: $registry (not upgraded?)"
  [[ "$contracts" == *"$address"* ]] || fail "$member (hash-$address) is not in NameToken.get_dapp_contracts (run the register-dapp scenario)"
  is_member=$(call NameToken is_dapp_contract --dapp_contract "hash-$address")
  [ "$is_member" = "true" ] || fail "NameToken.is_dapp_contract(hash-$address) returned: $is_member"
  ok "$member points at the NameToken and is a registered member"
done

echo "==> Metadata"
metadata=$(call NameToken get_dapp_metadata)
echo "$metadata" | sed 's/^/  /'
if echo "$metadata" | grep -q '"name": ""'; then
  echo "  [warn] the dapp name is empty; set it with: scenario register-dapp --name .."
fi

echo "==> All checks passed"
