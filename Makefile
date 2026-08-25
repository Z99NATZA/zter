# Development
run:
	cargo run

settings:
	cargo run -- settings apply

reload:
	cargo run -- settings reload

install:
	./scripts/install-dev-user.sh

# Build release
rel-settings:
	cargo run --release -- settings apply

rel-reload:
	cargo run --release -- settings reload

rel-install:
	./scripts/install-user.sh

rel-rm:
	./scripts/uninstall-user.sh
