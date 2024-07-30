default:
    just -l

deploy-example:
    cargo odra build
    cargo run --bin deploy-example -F livenet

lint:
    cargo fmt

show-structs-as-bytes:
    cargo run --bin structs-as-bytes