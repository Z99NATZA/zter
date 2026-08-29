# Run

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
zter settings apply
zter settings reload

# Standalone shortcut
zter -s

# Version
zter -v
```

## (Optional) Development

```bash
# icon, metadata
./scripts/install-dev-user.sh

# Run app from any path
zter-devel
```
