# External Monitor Brightness Applet for the COSMIC™ desktop

Change brightness of external monitors via DDC/CI protocol. You can also quickly toggle system dark mode.

![Screenshot](res/screenshot1.png)

## Install

### Fedora

You can use this [copr](https://copr.fedorainfracloud.org/coprs/wiiznokes/cosmic-applets-unofficial/).

```sh
sudo dnf copr enable wiiznokes/cosmic-applets-unofficial
sudo dnf install cosmic-ext-applet-external-monitor-brightness
```

### Other distros

```sh
sudo apt install just cargo libxkbcommon-dev git-lfs
git clone https://github.com/cosmic-utils/cosmic-ext-applet-external-monitor-brightness.git
cd cosmic-ext-applet-external-monitor-brightness
just build-release
sudo just install
```

## Troubleshooting

Maybe you need to setup the necessary udev rules if ddcutil is old.
For this to work you need write access to `/dev/i2c-*`.
See [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/).

## Credits

Originally created by [@maciekk64](https://github.com/maciekk64)
