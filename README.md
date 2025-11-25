# External Monitor Brightness Applet for the COSMIC™ desktop

Change brightness of external monitors via DDC/CI protocol. Native support for Apple Studio Display and LG UltraFine displays via USB HID. Includes automatic brightness synchronization with COSMIC brightness keys. You can also quickly toggle system dark mode.

![Screenshot](res/screenshot1.png)

## Features

- **DDC/CI Support**: Control brightness for standard external monitors using the DDC/CI protocol
- **Apple Studio Display & LG UltraFine**: Native USB HID support for Apple displays
  - Direct brightness control via applet slider
  - Automatic brightness sync daemon that mirrors COSMIC brightness keys (F1/F2) to Apple displays
  - Initial brightness sync on startup
  - Monitor name labels for easy identification
- **Dark Mode Toggle**: Quickly toggle system dark mode
- **Async Architecture**: Non-blocking UI with responsive controls
- **Protocol-Based Architecture**: Modular design supporting multiple display protocols simultaneously

## Installation

### Building from Source

```bash
# Build with default features (includes Apple Studio Display support and brightness sync daemon)
cargo build --release

# Build without Apple Studio Display support
cargo build --release --no-default-features --features brightness-sync-daemon

# Build with Apple support but without brightness sync daemon
cargo build --release --no-default-features --features apple-studio-display
```

### Feature Flags

- `apple-studio-display` (default): Enables USB HID support for Apple Studio Display and LG UltraFine displays
- `brightness-sync-daemon` (default): Enables automatic brightness synchronization with COSMIC brightness keys (F1/F2)

## Troubleshooting

### DDC/CI Displays

Maybe you need to setup the necessary udev rules if ddcutil is old.
For this to work you need write access to `/dev/i2c-*`.
See [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/).

### Apple Studio Display & LG UltraFine

On Linux, you need to set up udev rules to allow non-root access to the display's USB HID interface:

```bash
# Copy the udev rules file
sudo cp 99-apple-studio-display.rules /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

After installation, you may need to unplug and replug the display, or reboot your system for the changes to take effect.

If you see permission errors in the logs, ensure the udev rules are properly installed.

#### Brightness Sync Daemon

The brightness sync daemon automatically starts when Apple displays are detected. It:
- Listens to COSMIC brightness changes (F1/F2 keys)
- Syncs brightness to all connected Apple displays
- Applies the current COSMIC brightness on startup
- Runs in the background as a lightweight daemon

You can check if the daemon is running with:
```bash
RUST_LOG=info cosmic-ext-applet-external-monitor-brightness 2>&1 | grep daemon
```

Expected output:
```
INFO Found 1 Apple HID display(s), enabling brightness sync daemon
INFO Starting brightness sync daemon
INFO Connected to COSMIC Settings Daemon
INFO Initial sync: applying 60% to Apple displays (COSMIC value: 57600/96000)
INFO Listening for COSMIC brightness changes...
```

## Credits

Originally created by [@maciekk64](https://github.com/maciekk64)
