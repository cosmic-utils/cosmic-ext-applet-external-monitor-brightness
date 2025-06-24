# External Monitor Brightness Applet for the COSMIC™ desktop

Change brightness of external monitors via DDC/CI protocol. You can also quickly toggle system dark mode.

![Screenshot](res/screenshot1.png)

## Troubleshooting

Maybe you need to setup the necessary udev rules if ddcutil is old.
For this to work you need write access to `/dev/i2c-*`.
See [https://www.ddcutil.com/i2c_permissions/](https://www.ddcutil.com/i2c_permissions/).

## Credits

Originally created by [@maciekk64](https://github.com/maciekk64)
