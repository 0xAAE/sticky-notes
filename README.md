# Sticky Notes

This project is a remake of [indicator-stickynotes](https://github.com/umangv/indicator-stickynotes) using [pop-os/libcosmic](https://github.com/pop-os/libcosmic/tree/master) and targeting the [Cosmic DE](https://github.com/pop-os/cosmic-epoch)

There are two components in sticky-notes
* *notes-service* is a core application to deal with notes and settings
* *notes-applet* is an applet in Cosmic Panel providing main menu for *notes-service*

Other details is to be provided when version 0.1.0 has come

## Configuration

The path to configuration is `~/.config/cosmic/com.github.aae/sticky_notes/v1`.

Each value is stored in a separate file having following names.

### service_bin
:exclamation: highly desired

To provide a full pathname to *notes-service* binary file. It is used by the *notes-applet* to automatically launch the *notes-service* if it is not detected after start.

Value type: `string` (i.e. surrounded with double quotes)

Example: `"/home/user/.bin/notes-service"`

Default value: `"/usr/local/bin/notes-service"`

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

Value type: integer

Example: `32`

Default value: `16`

### `notes`
:exclamation: auto generated

Contains sticky-notes database.

Value type: `JSON string` (i.e. in double quotes).

:exclamation: Edit carefully otherwise it won't be read properly. It is highly recommended to edit notes in sticky windows and settings


## Build, install and run (current version only)

There are two components must start

* notes-service
* notes-applet

:point_up: The *notes-applet* automatically launches *notes-service* if it is not detected and if **service_bin** parameter is set in config. User might setup **service_bin** once and require to launch only *notes-applet* then.

A [justfile](./justfile) is included by default for the [casey/just][just] command runner.

- `just` does the same as `just rund-applet`
- `just rund-service` builds and runs the *notes-service* with debug profile
- `just rund-applet` builds and runs the *notes-applet* with debug profile
- `just run-service` builds and runs the *notes-service* with release profile
- `just run-applet` builds and runs the *notes-applet* with release profile
- `just debug` builds both *notes-applet* and *notes-service* with debug profile
- `just release` builds both *notes-applet* and *notes-service* with release profile
- `just install` installs the project into the system
- `just check` runs clippy on the project to check for linter warnings
- `cargo test` performs all unit tests

[just]: https://github.com/casey/just
