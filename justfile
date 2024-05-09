default:
    just -l

deploy-example:
    cargo run --bin deploy-example -F livenet