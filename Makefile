# Development
run:
	cargo run

settings:
	cargo run -- settings apply

reload:
	cargo run -- settings reload

install:
	./scripts/install-dev-user.sh

alone:
	cargo run -- -s

# Build release
rel-settings:
	zter settings apply

rel-reload:
	zter settings reload

rel-install:
	./scripts/install-user.sh

rel-rm:
	./scripts/uninstall-user.sh

rel-alone:
	zter -s