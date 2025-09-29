# `autokbdlight`

A small and simple daemon that turns on the keyboard backlight while the keyboard or trackpad are being used.

This can be used on laptops that don't implement this functionality in firmware, and instead leave it to the OS (for example: MacBooks, some ThinkPads, and some ASUS laptops).

In order to function, `autokbdlight` requires that the keyboard backlight is exposed as an LED under `/sys/class/leds`.

## Usage

The program is intended to be run as a systemd service.
To that end, a [service file](etc/systemd/autokbdlight.service) is provided.

It can take some optional command-line flags, shown here:

```
$ autokbdlight --help
Automatic keyboard backlight daemon for Linux

Usage: autokbdlight [OPTIONS]

Options:
  -c, --config <CONFIG>          Path to the (optional) configuration file
  -v, --verbose                  Enables more verbose logging
      --brightness <BRIGHTNESS>  Sets the default LED brightness when no config file is used (0-100)
  -h, --help                     Print help
  -V, --version                  Print version
```

## Configuration

When the program doesn't pick up the right input and LED devices on its own, a configuration file can be passed to the daemon via `autokbdlight -c <config.toml>`.

An example configuration with inline documentation can be found at [config.example.toml](./config.example.toml) in the repository.

For many systems, no configuration is required and the daemon will automatically detect the correct input devices and LEDs.

If the automatic detection works for you, and you only want to adjust the LED brightness, you can also just pass `--brightness 25` as an argument.
