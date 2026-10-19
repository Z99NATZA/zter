# Shortcuts

The application actions below use configurable defaults. Their bindings,
matching modes, reserved combinations, and failure handling are documented in
[Settings](settings.md#key-bindings).

| Default | Action |
| --- | --- |
| `Ctrl+C` on the physical `C` key | Copy selected text |
| `Ctrl+V` on the physical `V` key | Paste clipboard text |
| `Ctrl+PageUp` | Previous tab |
| `Super+H` | Previous tab |
| `Ctrl+PageDown` | Next tab |
| `Super+L` | Next tab |
| `Ctrl+T` | New tab |
| `Ctrl+=` | Increase the active tab's font size |
| `Ctrl+-` | Decrease the active tab's font size |

The zoom-reset action remains available through `zoom_reset` in
[settings](settings.md#key-bindings), but has no default shortcut.

These terminal protections and mouse gestures remain fixed:

| Key or gesture | Action |
| --- | --- |
| `Ctrl+C` without a selection | Send interrupt to an idle shell; confirm for a foreground process |
| `Ctrl+D` | Send to an idle shell; confirm for a foreground process |
| `Ctrl+Z` | Send to an idle shell; confirm for a foreground process |
| `Ctrl+Shift+Z` | Send to an idle shell; suppress for a foreground process |
| `Ctrl+mouse wheel` | Increase or decrease the active tab's font size |
| `Ctrl+left-click` | Open an OSC 8 hyperlink with the system-default handler |
