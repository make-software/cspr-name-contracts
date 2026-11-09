default:
    just -l

deploy-example:
    cargo odra build
    cargo run --bin deploy-example -F livenet

lint:
    cargo fmt

show-structs-as-bytes:
    cargo run --bin structs-as-bytes

cli *ARGS:
    cargo run --bin casper-name-cli -- {{ARGS}}

rebuild-docs:
    mv docs/sequence-diagrams sequence-diagrams-tmp
    rm -rf docs
    cargo doc -p casper-name-contracts --lib --no-deps
    cp -r target/doc docs
    echo "<meta http-equiv=\"refresh\" content=\"0; url=casper_name_contracts\">" > docs/index.html
    mv sequence-diagrams-tmp docs/sequence-diagrams


# ---------------------------------------------------------------------------
# Dapp registry upgrade (see README, "Versions")
# ---------------------------------------------------------------------------

set positional-arguments

# Alternative contracts file for the CLI (its `-c` option), relative to the
# project root. Empty means the CLI default, `resources/<chain>-contracts.toml`.
# Set it as an environment variable (`CONTRACTS_TOML=resources/foo.toml just ..`)
# or as a just variable (`just CONTRACTS_TOML=resources/foo.toml <recipe> ..`).
# It is exported, so the nested `just` calls in `upgrade-dapp-registry` see it.
export CONTRACTS_TOML := env("CONTRACTS_TOML", "")
cli_opts := if CONTRACTS_TOML == "" { "" } else { "-c " + CONTRACTS_TOML }
cli_cmd := "cargo run -q --bin casper-name-cli -- " + cli_opts

# Gas attached to every `upgrade` transaction.
gas := env("GAS", "400cspr")

# FRESH_INSTALL=1: the packages were installed with Odra 2.3 or newer (for
# example a fresh nctl deployment), so they already have an upgrade group and
# use the default package named keys. Without it the flags match the public
# deployment, whose packages were installed with an older Odra: the upgrade
# group is created on this first upgrade (the NameToken already has one), and
# the ReverseResolver package hash lives under a legacy named key.
fresh_install := env("FRESH_INSTALL", "0")
upgrade_group_flags := if fresh_install == "1" { "" } else { "--force-create-upgrade-group" }
reverse_resolver_flags := if fresh_install == "1" { "" } else { "--package-named-key ReverseResolver_contract_package" }

# Build the wasm files.
build:
    cargo odra build

# Upgrade one deployed contract with its wasm. Extra FLAGS go to `upgrade`.
upgrade CONTRACT *FLAGS:
    {{cli_cmd}} upgrade {{CONTRACT}} --gas {{gas}} {{FLAGS}}

# ARGS: --name, --description, --website_url, --icon_url.

# Set the dapp metadata and register the deployed members (idempotent).
register-dapp *ARGS:
    {{cli_cmd}} scenario register-dapp "$@"

# ARGS go to `register-dapp`. Run once, as the account that installed the
# contracts. SKIP_BUILD=1 reuses ./wasm.

# Upgrade every contract to the dapp registry version, then register the members.
upgrade-dapp-registry *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${SKIP_BUILD:-0}" != "1" ]; then
        just build
    fi
    # The Controller reads the registry from the Registrar during its upgrade,
    # and `register-dapp` calls `get_dapp_registry` on every member, which only
    # the upgraded code exposes.
    # CONTRACTS_TOML, FRESH_INSTALL and GAS reach the nested calls through the
    # environment.
    just upgrade NameToken
    just upgrade Registrar {{upgrade_group_flags}}
    just upgrade Controller {{upgrade_group_flags}}
    just upgrade DefaultResolver {{upgrade_group_flags}}
    just upgrade ReverseResolver {{upgrade_group_flags}} {{reverse_resolver_flags}}
    just register-dapp "$@"
    echo "==> Done. Verify with: CONTRACTS_TOML='{{CONTRACTS_TOML}}' just verify-dapp-registry"

# Exits non-zero on the first failed check.

# Verify (read-only) that the contracts are upgraded and registered in the dapp.
verify-dapp-registry:
    #!/usr/bin/env bash
    set -euo pipefail
    cli() { {{cli_cmd}} --json "$@"; }
    # Prints the `result` field of a `contract` call, or the error text.
    call() {
        cli contract "$@" 2>&1 | python3 -c '
    import json, sys
    lines = sys.stdin.read().splitlines()
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
    for c in json.load(sys.stdin)["registered"]:
        if c["ident"] == sys.argv[1] and c["deployed"]:
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
        [[ "$contracts" == *"$address"* ]] || fail "$member (hash-$address) is not in NameToken.get_dapp_contracts (run: just register-dapp)"
        is_member=$(call NameToken is_dapp_contract --dapp_contract "hash-$address")
        [ "$is_member" = "true" ] || fail "NameToken.is_dapp_contract(hash-$address) returned: $is_member"
        ok "$member points at the NameToken and is a registered member"
    done

    echo "==> Metadata"
    metadata=$(call NameToken get_dapp_metadata)
    echo "$metadata" | sed 's/^/  /'
    if echo "$metadata" | grep -q '"name": ""'; then
        echo "  [warn] the dapp name is empty; set it with: just register-dapp --name .."
    fi
    echo "==> All checks passed"
