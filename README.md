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
