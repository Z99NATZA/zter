# Development
settings:
	cargo run -- settings apply

run:
	cargo run

install:
	./scripts/install-dev-user.sh

# Build release
settings-rel:
	cargo run --release -- settings apply

install-rel:
	./scripts/install-user.sh

remove-rel:
	./scripts/uninstall-user.sh
