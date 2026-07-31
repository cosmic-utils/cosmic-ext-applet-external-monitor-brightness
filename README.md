# External Monitor Brightness Applet for the COSMIC™ desktop

Change brightness of external monitors via DDC/CI protocol. You can also quickly toggle system dark mode.

![Screenshot](res/screenshot1.png)

## Troubleshooting

Maybe you need to setup the necessary udev rules if ddcutil is old.
For this to work you need write access to `/dev/i2c-*`.
See [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/).

## Logs

```sh
journalctl -p 3 -xb --user _EXE=/usr/bin/cosmic-ext-applet-external-monitor-brightness | less
```

- `-p` 3 means priority error
- `-x` add information
- `b` means since last boot

## Testing bundle

```sh
# install
flatpak install --user external-monitor-brightness.flatpak
# run specific branch
flatpak run --branch=testing io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness
# to be sure cosmic-panel will launch the wanted version
flatpak uninstall --user io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness//master
# or verify the commit with
flatpak run io.github.cosmic_utils.cosmic-ext-applet-external-monitor-brightness -V
# uninstall testing repo and app
flatpak remote-delete --user cosmic-ext-applet-external-monitor-brightness-origin
```

## Build from source

Instructions are in [this file](./BUILD.md).

## Contributing

See [this file](./CONTRIBUTING.md).

## Credits

Originally created by [@maciekk64](https://github.com/maciekk64)
