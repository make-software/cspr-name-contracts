# CSPR.name Contracts

## Usage
It's recommended to install 
[cargo-odra](https://github.com/odradev/cargo-odra) first.

### Prerequisites

Install wasm-opt tool (it is needed for building contracts)
```
$ cargo install wasm-opt --locked
```

if you run casper node localy (nctl), you need to copy files to have access to the accounts keys from the host
```
$ docker run --rm -it --name mynctl -d -p 11101:11101 -p 14101:14101 -p 18101:18101 makesoftware/casper-nctl

$ docker cp mynctl:/home/casper/casper-node/utils/nctl/assets/net-1 .
```

Copy example env file and make appropriate adjustments (default values work with nctl)
```
$ cp .env.example .env
```

### Build

```
$ cargo odra build
```
To build a wasm file, you need to pass the -b parameter. 
The result files will be placed in `${project-root}/wasm` directory.

```
$ cargo odra build -b casper
```

### Deploy

```
$ cargo run -p casper-name-cli deploy
```

### Test
To run test on your local machine, you can basically execute the command:

```
$ cargo odra test
```

To test actual wasm files against a backend, 
you need to specify the backend passing -b argument to `cargo-odra`.

```
$ cargo odra test -b casper
```

# Versions

## Dapp registry

The contracts are grouped into one dapp with the `odra_modules::dapp` modules
(Odra 2.10.0). The NameToken is the registry, and the other contracts are its
members.

### What changes

- `NameToken` composes `DappRegistryBase`. New entry points:
  `set_dapp_metadata`, `add_dapp_contract` and `remove_dapp_contract`
  (owner only), and the read-only `get_dapp_metadata`, `get_dapp_contracts`,
  `is_dapp_contract`, `is_dapp_registrar` and `get_dapp_registry`.
- `Registrar`, `DefaultResolver`, `ReverseResolver`, `Controller` and
  `SecondaryMarket` compose `DappContractBase`. New entry points:
  `get_dapp_registry`, and `set_dapp_registry` guarded by the contract's admin
  (the NameToken owner for the `ReverseResolver`, which has no admin of its
  own, error `NotNameTokenOwner` = 1504).
- Every contract gets a no-argument `upgrade` entry point. On the NameToken it
  creates the registry storage. On a member it stores the NameToken address
  as the registry (the Controller asks its Registrar for it).
- New events `DappMetadataChanged`, `DappContractAdded`, `DappContractRemoved`
  on the NameToken and `DappRegistryChanged` on the members. The dapp errors
  use codes 22000 to 22006.
- The new storage is appended after the existing fields, under the `__dapp`
  named key, so existing tokens, resolutions, primary names and roles are
  untouched. `init` signatures and existing entry points do not change.
- A new `register-dapp` CLI scenario sets the metadata and adds every
  deployed member that is not registered yet. It is safe to rerun.
- New `just` recipes: `upgrade`, `register-dapp`, `upgrade-dapp-registry`
  and `verify-dapp-registry`.

### Commands to run

Run them as the account that installed the contracts, with the livenet
configuration of the target network selected (`ODRA_CASPER_LIVENET_ENV=<name>`
picks `<name>.env`). The whole sequence is one `just` recipe:

```
$ just upgrade-dapp-registry --name "CSPR.name" \
    --description "Casper Name Service" \
    --website_url "https://cspr.name" --icon_url "https://cspr.name/icon.png"
```

It builds the wasm files, upgrades the five contracts and runs
`register-dapp`. The order matters: the Controller reads the registry from the
Registrar during its upgrade, and `register-dapp` calls `get_dapp_registry` on
each member, which only the upgraded code has. Step by step, it runs:

```
$ just build
$ just upgrade NameToken
$ just upgrade Registrar --force-create-upgrade-group
$ just upgrade Controller --force-create-upgrade-group
$ just upgrade DefaultResolver --force-create-upgrade-group
$ just upgrade ReverseResolver --force-create-upgrade-group \
    --package-named-key ReverseResolver_contract_package
$ just register-dapp --name "CSPR.name" \
    --description "Casper Name Service" --website_url "https://cspr.name"
```

`just upgrade <Contract> [flags]` wraps
`cargo run --bin casper-name-cli -- upgrade <Contract> --gas 400cspr [flags]`.
The CLI reads `resources/<chain>-contracts.toml`; to use another contracts
file, pass it to any of these recipes as a `just` variable, for example
`just contracts_toml=resources/casper-test-contracts.toml upgrade-dapp-registry ..`
(or set `CONTRACTS_TOML`). It becomes the CLI's `-c` option.

`--force-create-upgrade-group` is for packages installed with Odra 2.2 or
earlier that have never been upgraded, which is the state of the public
deployment (the NameToken was upgraded before and already has the group). A
package that already has the group fails with `Contract header error: 3`.
For contracts installed with Odra 2.3 or newer, such as a fresh nctl
deployment, run the recipe with `FRESH_INSTALL=1`, which drops the flag and
the legacy named key. `SKIP_BUILD=1` reuses the wasm files in `./wasm`, and
`GAS` overrides the 400 CSPR per upgrade.

### How to verify

```
$ just verify-dapp-registry
```

The recipe only reads. It checks that the NameToken reports itself as the
registry and first in `get_dapp_contracts`, that every deployed member
reports the NameToken from `get_dapp_registry` and is listed as a member, and
prints the metadata. It exits non-zero on the first failed check. By hand:

```
$ cargo run -p casper-name-cli -- contract NameToken get_dapp_contracts
Call result: [Key::Hash(<NameToken>),Key::Hash(<DefaultResolver>),Key::Hash(<Registrar>),Key::Hash(<Controller>),Key::Hash(<ReverseResolver>)]
$ cargo run -p casper-name-cli -- contract Registrar get_dapp_registry
Call result: Key::Hash(<NameToken>)
$ cargo run -p casper-name-cli -- contract NameToken get_dapp_metadata
$ cargo run -p casper-name-cli -- print-events NameToken -n 5
```

The hashes are the package hashes from `status`. The last command shows the
`DappMetadataChanged` and `DappContractAdded` events written by
`register-dapp`. A member that was not upgraded answers `get_dapp_registry`
with `DappRegistryNotSet`; one that was upgraded but not registered is
missing from `get_dapp_contracts`. The transaction result of each `upgrade`
command is the proof that the new code is live, since getters run the local
code against the chain state.
