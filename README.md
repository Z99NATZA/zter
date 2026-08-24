# zter

```text
Project created at 2026-08-24
- README.md updated at 2026-08-24
- Developed on Ubuntu
```

## Installation

```bash
# Rust/Cargo
sudo apt install curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Ubuntu dependencies 
sudo apt install build-essential pkg-config libgtk-4-dev libvte-2.91-gtk4-dev

```

## Run

```bash
# Clone
git clone https://github.com/Z99NATZA/zter.git
cd zter

# Apply current settings (Optional)
cargo run -- settings apply

# Run
cargo run
```

## Build release to Application desktop

```bash
# Install
./scripts/install-user.sh

# Remove
./scripts/uninstall-user.sh
```

## Documentation

- [Settings](docs/settings.md)
- [Terminal runtime](docs/terminal-runtime.md)
- [Desktop integration](docs/desktop-integration.md)
