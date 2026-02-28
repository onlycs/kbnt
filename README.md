# KBNT - **K**ey**b**oard over **N**etwork**T**ables

A small tool used to pipe some keypress (keydown) events over NetworkTables
while the robot is connected and the driver station is open.

## Usage

1. Run the `kbnt.exe` executable, from the releases tab
2. The binary will automatically burrow itself among your local AppData[^1] and start running in the background
3. Edit the config file[^2] to your liking and save it
4. Open the driver station, connect to the robot, and use your favorite NT viewer (e.g. AdvantageScope).

## Notes

1. "Hot reloading" of the config file is supported... to an extent. An active robot connection will never be interrupted or modified[^3] by a config change, but all changes take effect on all future connection attempts.
2. Error handling is done by putting a log file in the same directory as the binary. You will get a notification on errors
3. You will also get notified on
    - app startup
    - driverstation launch
    - robot connection
    - robot disconnection, followed by status (i.e. waiting for DS or robot)
4. The app will forever run in the background of your machine. On first run, it will set itself up to start on login. See `src/install.rs` for how this works.

## Robot Code

Coming as soon as I get this tested on a windows computer. (Hint: I use nix, btw).

## Why our team is using this

We recently acquired some Xbox Super Series 2 Elite Pro Max Pluses or whatevers ([this](https://www.amazon.com/Elite-2-Controller-Black-Xbox-One/dp/B07SFKTLZM)) with paddles on the back. Since the handheld controller HID model doesn't support extra buttons, we have to configure the paddles to emulate keypresses.

[^1]: The binary gets copied to `AppData\Local\team2791\kbnt\kbnt.exe` as well as an example config file.

[^2]: The config file is stored in the same folder as the binary

[^3]: We will not reconnect based on an IP change, nor change the keys being sent during the connection. Sorry, Windows is a pain to code on
