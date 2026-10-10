.PHONY: test-unit test-integrated test-system test-all lint fmt run serve check golden balance assess

test-unit:
	cargo test --workspace --lib --bins

test-integrated:
	cargo test --workspace --test '*'

test-system:
	cargo test --workspace -- --ignored

test-all: test-unit test-integrated test-system

lint:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings

fmt:
	cargo fmt --all

check: lint test-unit test-integrated

run:
	cargo run -p goblinsbowl --features bevy/dynamic_linking -- $(ARGS)

serve:
	cargo run -p goblins_server -- $(ARGS)

golden:
	cargo run -p goblins_rules --example record_golden_match

balance:
	cargo run --release -q -p goblins_rules --example balance -- $(ARGS)

assess:
	cargo run -q -p goblins_server --bin goblinsbowl-assess -- $(ARGS)
