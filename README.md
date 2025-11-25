# External Monitor Brightness Applet for the COSMIC™ desktop

Change brightness of external monitors via DDC/CI protocol. Native support for Apple displays (Studio Display, Pro Display XDR) and LG UltraFine displays via USB HID. Includes automatic brightness synchronization with COSMIC brightness keys (F1/F2). You can also quickly toggle system dark mode.

<img src="res/screenshot3.png" width="600" alt="Screenshot">

## Features

- **DDC/CI Support**: Control brightness for standard external monitors using the DDC/CI protocol
  - Fast concurrent enumeration for quick startup
  - Per-monitor brightness control with gamma curve adjustment (0.3-3.0 range)
  - Minimum brightness settings to prevent displays from going too dim
- **Apple HID Display Support**: Native USB HID support for Apple displays
  - Supported displays: Studio Display, Pro Display XDR, LG UltraFine 4K/5K
  - Device-specific default gamma curves (1.8 for Apple displays, optimized for their native brightness response)
  - Direct brightness control via applet slider
  - Monitor name labels for easy identification
- **F1/F2 Brightness Key Sync**: Automatic brightness synchronization with COSMIC brightness keys
  - Works with both DDC/CI and Apple HID displays
  - Per-monitor toggle to enable/disable sync
  - Configurable sync mode (all displays or primary only)
  - Lightweight background daemon
- **Enhanced UI**:
  - Icons in settings menu for better visual organization
  - Precise gamma control with +/- buttons (0.1 increments)
  - Clear display of current values
  - Monitor name labels for multi-monitor setups
- **Dark Mode Toggle**: Quickly toggle system dark mode
- **Async Architecture**: Non-blocking UI with responsive controls
- **Protocol-Based Architecture**: Modular design supporting multiple display protocols simultaneously

## Installation

### Dependencies

#### Build Dependencies
- Rust toolchain (cargo)
- System libraries:
  - `i2c-dev` headers (for DDC/CI support)
  - `hidapi` development files (for Apple HID display support)

#### Runtime Dependencies
- **For DDC/CI displays** (standard external monitors):
  - `i2c-tools` or `ddcutil`
  - User must be in the `i2c` group
- **For Apple HID displays** (Studio Display, Pro Display XDR, LG UltraFine):
  - `hidapi` library
  - Proper udev rules installed (see Troubleshooting section)

#### Package names by distribution:

**Fedora/RHEL:**
```bash
# Build dependencies
sudo dnf install rust cargo i2c-tools-devel hidapi-devel

# Runtime dependencies
sudo dnf install i2c-tools hidapi ddcutil
```

**Debian/Ubuntu:**
```bash
# Build dependencies
sudo apt install cargo libi2c-dev libhidapi-dev

# Runtime dependencies
sudo apt install i2c-tools libhidapi-libusb0 ddcutil
```

**Arch Linux:**
```bash
# Build dependencies
sudo pacman -S rust i2c-tools hidapi

# Runtime dependencies (same as build)
sudo pacman -S i2c-tools hidapi ddcutil
```

### Building from Source

```bash
# Build with default features (includes Apple HID display support)
cargo build --release

# Build without Apple HID display support (DDC/CI only)
cargo build --release --no-default-features
```

### Feature Flags

- `apple-hid-displays` (default): Enables USB HID support for Apple displays (Studio Display, Pro Display XDR) and LG UltraFine displays

## Troubleshooting

### DDC/CI Displays

You may need to set up udev rules for I2C access if you encounter permission errors.
For this to work you need write access to `/dev/i2c-*`.

```bash
# Copy the I2C permissions udev rules
sudo cp data/udev/45-i2c-permissions.rules /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Alternatively, see [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/) for more information.

### Apple HID Displays (Studio Display, Pro Display XDR, LG UltraFine)

On Linux, you need to set up udev rules to allow non-root access to the display's USB HID interface:

```bash
# Copy the Apple HID udev rules
sudo cp data/udev/99-apple-displays.rules /etc/udev/rules.d/

# Reload udev rules
sudo udevadm control --reload-rules
sudo udevadm trigger
```

After installation, you may need to unplug and replug the display, or reboot your system for the changes to take effect.

If you see permission errors in the logs, ensure the udev rules are properly installed.

### F1/F2 Brightness Key Synchronization

The brightness sync daemon automatically syncs COSMIC brightness keys (F1/F2) to external monitors. Features:
- Works with both DDC/CI and Apple HID displays
- Per-monitor toggle to enable/disable sync (right-click on monitor icon in settings)
- Syncs on startup and when brightness keys are pressed
- Runs in the background as a lightweight daemon

You can disable sync for specific monitors by:
1. Right-click the monitor icon to open settings
2. Toggle the "Sync with F1/F2 brightness keys" switch

You can check if the daemon is running with:
```bash
RUST_LOG=info cosmic-ext-applet-external-monitor-brightness 2>&1 | grep daemon
```

## Credits

Originally created by [@maciekk64](https://github.com/maciekk64)

Apple HID protocol implementation based on [asdbctl](https://github.com/juliuszint/asdbctl) by @juliuszint
