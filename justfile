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
#
# Every recipe takes the contracts file and the gas explicitly, and the
# per-contract recipes pass any further flags straight to the CLI's `upgrade`
# command, so the command line shows exactly what is sent. Nothing is read
# from the environment.
#
#   CONTRACTS_TOML  contracts file of the target network, relative to the
#                   project root, e.g. resources/casper-contracts.toml
#   GAS             gas per transaction, in motes or as `400cspr`
#   FLAGS           `--force-create-upgrade-group` for a package installed with
#                   Odra 2.2 or earlier that has never been upgraded,
#                   `--package-named-key <NAME>` for a package whose hash lives
#                   under a non-default named key (see `upgrade <Contract> --help`)
# ---------------------------------------------------------------------------

set positional-arguments
set shell := ["bash", "-c"]

cli_cmd := "cargo run -q --bin casper-name-cli --"

# Build the wasm files.
build:
    cargo odra build

# Upgrade the NameToken with wasm/NameToken.wasm. FLAGS go to `upgrade`.
upgrade-name-token CONTRACTS_TOML GAS *FLAGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} upgrade NameToken --gas {{GAS}} {{FLAGS}}

# Upgrade the Registrar with wasm/Registrar.wasm. FLAGS go to `upgrade`.
upgrade-registrar CONTRACTS_TOML GAS *FLAGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} upgrade Registrar --gas {{GAS}} {{FLAGS}}

# Upgrade the Controller with wasm/Controller.wasm. FLAGS go to `upgrade`.
upgrade-controller CONTRACTS_TOML GAS *FLAGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} upgrade Controller --gas {{GAS}} {{FLAGS}}

# Upgrade the DefaultResolver with wasm/DefaultResolver.wasm. FLAGS go to `upgrade`.
upgrade-default-resolver CONTRACTS_TOML GAS *FLAGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} upgrade DefaultResolver --gas {{GAS}} {{FLAGS}}

# Upgrade the ReverseResolver with wasm/ReverseResolver.wasm. FLAGS go to `upgrade`.
upgrade-reverse-resolver CONTRACTS_TOML GAS *FLAGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} upgrade ReverseResolver --gas {{GAS}} {{FLAGS}}

# ARGS: --name, --description, --website_url, --icon_url.

# Set the dapp metadata and register the deployed members (idempotent).
register-dapp CONTRACTS_TOML *ARGS:
    {{cli_cmd}} -c {{CONTRACTS_TOML}} scenario register-dapp "${@:2}"

# MODE is `legacy` for the public deployment (packages installed with Odra 2.2
# or earlier: the upgrade group is created now, except for the NameToken which
# already has one, and the ReverseResolver uses its legacy named key) or
# `fresh` for packages installed with Odra 2.3 or newer, such as a new nctl
# deployment. ARGS go to `register-dapp`. Run `just build` first, and run this
# once, as the account that installed the contracts.

# Upgrade every contract in the required order, then register the members.
upgrade-all CONTRACTS_TOML GAS MODE *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{MODE}}" in
        legacy)
            group="--force-create-upgrade-group"
            key="--package-named-key ReverseResolver_contract_package"
            ;;
        fresh)
            group=""
            key=""
            ;;
        *)
            echo "MODE must be 'legacy' or 'fresh', got '{{MODE}}'" >&2
            exit 2
            ;;
    esac
    # The Controller reads the registry from the Registrar during its upgrade,
    # and `register-dapp` calls `get_dapp_registry` on every member, which only
    # the upgraded code exposes.
    just upgrade-name-token {{CONTRACTS_TOML}} {{GAS}}
    just upgrade-registrar {{CONTRACTS_TOML}} {{GAS}} $group
    just upgrade-controller {{CONTRACTS_TOML}} {{GAS}} $group
    just upgrade-default-resolver {{CONTRACTS_TOML}} {{GAS}} $group
    just upgrade-reverse-resolver {{CONTRACTS_TOML}} {{GAS}} $group $key
    just register-dapp {{CONTRACTS_TOML}} "${@:4}"
    echo "==> Done. Verify with: just verify-dapp-registry {{CONTRACTS_TOML}}"

# Exits non-zero on the first failed check.

# Verify (read-only) that the contracts are upgraded and registered in the dapp.
verify-dapp-registry CONTRACTS_TOML:
    #!/usr/bin/env bash
    set -euo pipefail
    cli() { {{cli_cmd}} -c {{CONTRACTS_TOML}} --json "$@"; }
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

    echo "==> Deployed contracts ({{CONTRACTS_TOML}})"
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
        [[ "$contracts" == *"$address"* ]] || fail "$member (hash-$address) is not in NameToken.get_dapp_contracts (run: just register-dapp {{CONTRACTS_TOML}})"
        is_member=$(call NameToken is_dapp_contract --dapp_contract "hash-$address")
        [ "$is_member" = "true" ] || fail "NameToken.is_dapp_contract(hash-$address) returned: $is_member"
        ok "$member points at the NameToken and is a registered member"
    done

    echo "==> Metadata"
    metadata=$(call NameToken get_dapp_metadata)
    echo "$metadata" | sed 's/^/  /'
    if echo "$metadata" | grep -q '"name": ""'; then
        echo "  [warn] the dapp name is empty; set it with: just register-dapp {{CONTRACTS_TOML}} --name .."
    fi
    echo "==> All checks passed"
