# Run

## (Optional) Apply current settings

```bash
cargo run -- settings apply
cargo run
```

## (Optional) Build release to Application desktop

```bash
# Install
# after installation, the zter application will appear on your desktop
# and you can launch zter from any path
./scripts/install-user.sh

# ------------------------------

# Run app from any path
zter

# ------------------------------

# Remove
./scripts/uninstall-user.sh

# ------------------------------

# Apply current settings
cargo run --release -- settings apply
./scripts/uninstall-user.sh
./scripts/install-user.sh
```

## (Optional) Development

```bash
# icon, metadata
./scripts/install-dev-user.sh

# Run app from any path
zter-devel
```
