# Sticky Notes
## Stickers with urgent tasks that won't come off. Even after years.

This project is a remake of [indicator-stickynotes](https://github.com/umangv/indicator-stickynotes) using [pop-os/libcosmic](https://github.com/pop-os/libcosmic/tree/master) and targeting the [Cosmic DE](https://github.com/pop-os/cosmic-epoch) but... It allows **using markdown** to display formatted content and open URL links by click.

There are two components in sticky-notes
* *notes-service* is a core application to deal with notes and settings
* *notes-applet* is an applet in Cosmic Panel providing main menu for *notes-service*

![example](resources/doc/screen-01.png)

## Quick start

### Prerequisites

#### Common

* [rust]. The recommended way to install with rustup is a good choice: 
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
* [casey/just][just]. Because the project sticky-notes is built with cargo and rust compiler the good choice to install just is 
```
cargo install just
```

#### Ubuntu 24.04, Pop!_OS 24.04

* library xkbcommon (Otherwise the compilation error will occur: "The system library `xkbcommon` required by crate `smithay-client-toolkit` was not found"). The command to install the library in Ubuntu or Pop!_OS
```
sudo apt install libxkbcommon-dev
```

### Build from source code

* Open terminal
* Create working directory to host the project code and temporary files. Example
```
mkdir ~/build && cd ~/build
```
* Clone the project
```
git clone https://github.com/0xAAE/sticky-notes.git && cd sticky-notes
```
* Build release version

Cosmic DE
```
just build-cosmic
```
Wayland-based DE:
```
just build-wayland
```
X11-based DE:
```
just build-x
```

### Run for testing (optional)

* Run to test everything is good in terminal

Cosmic DE
```
just run
```
Wayland-based DE:
```
just run-wayland
```

X11-based DE:
```
just run-x
```

* Stop application working in terminal with `Ctrl-C`

### Install
* Install sticky-notes
```
sudo just install
```
* In Cosmic DE additionally install sticky-notes applet to place it into Cosmic Panel
```
sudo just install-applet
```

## Configuration

The path to configuration is `~/.config/cosmic/com.github.aae/sticky_notes/v1`.

Each value is stored in a separate file having following names.

### service_bin
:exclamation: highly desired

To provide a full pathname to *notes-service* binary file. It is used by the *notes-applet* to automatically launch the *notes-service* if it is not detected after start.

Value type: `string` (i.e. surrounded with double quotes)

Example: `"/home/user/.bin/notes-service"`

Default value: `"/usr/bin/notes-service"`

### connect_service_pause_ms
optional

When *notes-applet* starts it connects the *notes-service*. If connecting fails the applet launches the service (defined in *service_bin*), then it waits some time and retries to connect after that again. This parameter defines in milliseconds how long the applet will wait before retrying to connect the service.

Value type: `integer`

Example: `2_000`

Default value: `1_000`

### autosave_period_ms
optional

If this parameter is non zero it defines the period of time in milliseconds to autosave all changes in notes. If it is set to 0 autosave feature is off.

Value type: `integer`

Example: `0`

Default value: `30_000`

### import_file
optional

To provide a pathname to *indicator-stickynotes* database file relative to user's home directory. It is used for importing notes when
  * no database detected on startup
  * command `Import` selected in *notes-applet* menu

Value type: `string` (i.e. surrounded with double quotes)

Example: `".config/indicator-stickynotes"`

Default value: `".config/indicator-stickynotes"`

### restore_notes_width, restore_notes_height
optional

Overrides the width and height of the window to restore notes.

Value type: `integer`

Example: `1024`

Default values: restore_notes_width is `480` and restore_notes_height is `400`

### edit_style_width, edit_style_height
optional

Overrides the width and height of the window to edit selected note style.

Value type: `integer`

Example: `1024`

Default values: edit_style_width is `480` and edit_style_height is `800`

### about_width, about_height
optional

Overrides the width and height of the window to display application info.

Value type: `integer`

Example: `1024`

Default values: about_width is `480` and about_height is `840`

### note_min_width, mote_min_height
optional

Overrides the minimum width and the minimum height of the note sticky window.
If user manually violates minimum values (making window tiny) they will be accepted until the next start.
The next start the minimum values will be applied.
If default values are too large, one might override them setting these parameters.

Value type: `integer`

Example: `32`

Default values: note_min_width is `64` and mote_min_height is `64`

### toolbar_icon_size
optional

Overrides the size of icons in the sticky window toolbar.

Value type: `integer`

Example: `32`

Default value: `16`

### `notes`
:exclamation: auto generated

Contains sticky-notes database.

Value type: `JSON string` (i.e. in double quotes).

:exclamation: Edit carefully otherwise it won't be read properly. It is highly recommended to edit notes in sticky windows and settings

## Build, install and run

There are two components should start

* notes-service
* notes-applet

:point_up: The *notes-applet* automatically launches *notes-service* if it is not detected and if **service_bin** parameter is set in config. User might setup **service_bin** once in configuration then to launch only *notes-applet*.

A [justfile](./justfile) is included by default for the [casey/just][just] command runner.

- `just --list` briefly displays all recipes
- `just clean` cleans the project by removing all generated files
### Develop
- `just check` runs formatting then runs clippy on the project to check for linter warnings
- `just dbg` builds and runs debug version of *notes-service*
- `just dbg-applet` builds debug version of both *notes-applet* and *notes-service* then runs *notes-applet* which in its turn launches *notes-service* itself
- `cargo test` invokes provided unit tests
### Build
- `just build` builds release version of *notes-service* targeting the Cosmic DE
- `just build-cosmic` builds release version of both *notes-applet* and *notes-service* targeting the Cosmic DE
- `just build-wayland` builds release version of the *notes-service* targeting the Wayland-based DE
- `just build-x` builds release version of the *notes-service* targeting the X11-based DE

### Run
- `just` is the same as `just run`
- `just run` runs release version of the *notes-service* targeting the Cosmic DE
- `just run-cosmic` is the same as `just run`
- `just run-applet` runs release version of the *notes-applet* targeting the Cosmic DE
- `just run-wayland` runs release version of the *notes-service* targeting the Wayland-based DE
- `just run-x` runs release version of the *notes-service* targeting the X11-based DE
### Install
- `sudo just install` installs the project into the system
- `sudo just uninstall` uninstalls the project from the system

### Features

The features are to control the build process:

* `cosmic` (default) - to build both service and applet for running in Cosmic DE
* `wayland` - to build only service for running in Wayland-based environment other then Cosmic DE
* `x11` - to build only service for running in X11 Server

and to embed icons into binary:
* `embed-icons` - embed svg files into binary file and don't use XDG system-wide icons

## Troubleshooting

### Unable to install: just is not found

In some environments (example: Ubuntu) `sudo just` unable to find `just` if it is installed by `cargo install just` into user's home directory.

**Solution** (one of). Locate `just` binary
```
which just
```
output example: `/home/username/.cargo/bin/just`

Then, create link to just in shared directory
```
ln -s /home/username/.cargo/bin/just /usr/local/bin/just
```

### Icons on sticky window toolbar are absent, every or any of them

**Solution**. By default, the application uses [freedesktop] icons located in the predefined directories. If somehow icons could not be found it is possible to re-build application with feature `embed-icons` is on. The feature forces to embed all icons into binary so no icon looking up is required. In this case quick start is
```
just build-cosmic --features embed-icons
```
```
just run-cosmic --features embed-icons
```
```
sudo just install
```

[rust]: https://rust-lang.org/tools/install
[just]: https://github.com/casey/just
[freedesktop]: https://specifications.freedesktop.org/icon-theme/latest

### Sticky windows do not restore their positions on start

This is limitation from Wayland which allows to control only window size but not its position. Whereas, in X11 notes positions are restored normally.

### There is no Cosmic DE, how to launch

**Solution**. By default the application is built for running in Cosmic DE. To run in another Wayland-based desktop it is possible to build with feature `wayland`:
```
just build-wayland
```
```
just run-wayland
```
```
sudo just install
```
In this case, there is no Cosmic panel applet available but notes-service works like standalone application itself.

### There is no Wayland at all, how to launch

**Solution**. To run in X11-based desktop one should build with feature `x11`:
```
just build-x
```
```
just run-x
```
```
sudo just install
```

In this case, there is no Cosmic panel applet available but notes-service works like standalone application itself.

### To activate many build features

Build features could be combined while building the application binary. Example
```
just build --no-default-features --features x11,embed-icons
```
```
just run --no-default-features --features x11,embed-icons
```
```
sudo just install
```

### The system theme is light while some elements are displayed like in dark theme

**Solution**. On the first launch some config directories are created automatically. Locate the
`~/.config/cosmic/com.system76.CosmicTheme.Mode/v1` directory and enter it
```
cd ~/.config/cosmic/com.system76.CosmicTheme.Mode/v1
```
Then create file `is_dark` containing value false
```
echo false > is_dark
```
