settings:
	cargo run -- settings apply

settings-rel:
	cargo run --release -- settings apply

run:
	cargo run

install:
	./scripts/install-user.sh

remove:
	./scripts/uninstall-user.sh
