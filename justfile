default:
    just -l

deploy-example:
    cargo odra build
    cargo run --bin deploy-example -F livenet

lint:
    cargo fmt

show-structs-as-bytes:
    cargo run --bin structs-as-bytes

rebuild-docs:
	rm -rf docs
	cargo doc -p casper-name-contracts --lib --no-deps
	cp -r target/doc docs
	echo "<meta http-equiv=\"refresh\" content=\"0; url=casper_name_contracts\">" > docs/index.html