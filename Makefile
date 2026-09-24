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

HEADER_MODES := full mini hide show
.PHONY: header $(HEADER_MODES)

header:
	@mode='$(filter $(HEADER_MODES),$(MAKECMDGOALS))'; \
	case "$$mode" in \
		full|mini|hide|show) cargo run -- header "$$mode" ;; \
		*) echo 'Usage: make header {full|mini|hide|show}' >&2; exit 2 ;; \
	esac

$(HEADER_MODES): header
	@:

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
