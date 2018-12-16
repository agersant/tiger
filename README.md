Tiger is a graphical tool for generating spritesheets and metadata about the animation and hitboxes they contain.

![Tiger](res/readme/screenshot-0.1.0.png?raw=true "Tiger")

# Key Features

- Timeline-editing for authoring animations
- Easy to add and position hitboxes
- Support for custom formats when exporting metadata
- Generated texture atlas for use in-engine
- Free and open-source with a permissive license

# Getting Started

## Requirements

One of the following:
- Windows 7 or newer
- Linux (any reasonably modern distribution should do)

## Installation

### Windows
1. Download the [latest release](https://github.com/agersant/tiger/releases/latest) (you want the .exe file)
2. Run the executable
3. That's it, you're done!

### Linux

#### Dependencies

1. Install GTK-3. This is most likely available from your distribution's package manager. For instance on Ubuntu, execute `sudo apt-get install libgtk-3-dev`
2. Install the Rust nightly compiler by executing `curl https://sh.rustup.rs -sSf | sh -s --default-toolchain nightly` or using an [alternative method](https://www.rust-lang.org/en-US/install.html)

#### Tiger installation
1. Download the [latest release]((https://github.com/agersant/tiger/releases/latest)) of Tiger (you want the .tar.gz file)
2. Extract the archive in a directory and open a terminal in that directory
3. Execute `make install` (this may take several minutes)

This installation process puts the Tiger executable in `~/.local/bin/tiger`.

If you want to uninstall Tiger, execute `make uninstall` from the extracted archive's directory. This will simply delete the files created by the install process.

# Roadmap

## Tiger 0.1
	- ☑️ Don't store absolute paths in tiger files
	- ☑️ Show frame names in timeline
	- ☑️ Solution(s) to edit/order/insert frames in timeline
	- ☑️ Add, position, tag, delete hitboxes
	- ☑️ Full compat with crystal sheets:
		- ☑️ Hitboxes
		- ☑️ Sheet path in export
		- ☑️ Top left coords available in template
	- ☑️ Tiger backwards compat
	- ☑️ Release pipeline
	- ☑️ No placeholder menu options
	- ☑️ Export (using last known settings)
	- ☑️ Draw frame being dragged even during animation
	- ☑️ Draw hitboxes during animation
	- ☑️ Animation renames
	- ☑️ Allow user to choose what directory paths are relative to during export

	- ☑️ Fix bug where export window shows weird absolute + relative concatenated paths
	- ☑️ Fix bug where pressing delete while renaming an animation(/hitbox) deletes it
	- ☑️ Fix bug where renaming an animation(/hitbox) unselects and unedits it
	- ☑️ Fix bug where animation frame duration drag shows insert markers
	- ☑️ Fix bug where animation frames can be reorderer by dragging timeline
	- ☑️ Fix bugs when manipulating extremely short animation frames

## Tiger 0.2
	- ☐ Unsaved file marker and warnings
	- ☐ Undo/Redo
	- ☐ Keyboard shortcuts for playback controls
	- ☐ Keyboard shortcuts menu entries
	- ☐ Keyboard shortcuts for list navigation
	- ☐ Loading spinners
	- ☐ Begin editing animation after creating it
	- ☐ Select hitbox after creating it
	- ☐ Duplicate animation / animation frame (within same sheet)
	- ☐ Grid
	- ☐ Drag and drop frames to workbench
	- ☐ Grid snapping
	- ☐ Content of selection window when selecting animation frame
	- ☐ Content of selection window when selecting hitbox
	- ☐ In selection window, keep origin centered to preview turnarounds
	- ☐ When moving animation frame or hitbox, hold shift to move only on one axis
	- ☐ When resizing hitbox, hold shift to make square (or preserve aspect ratio?)
	- ☐ Content window and workbench should say what the current workbench item is
	- ☐ Sort content panel entries by name
	- ☐ Sort hitbox panel entries by name
	- ☐ Dont draw origin when editing frame
	- ☐ Use rect and point structs consistently instead of tuples everywhere

	- ☐ Fix bug where origin is not consistent within one animation in selection window (is ok in workbench)
	- ☐ Fix bug where frame name can go outside frame bound in timeline
	- ☐ Fix bug where reordering frame changes selected frame

## Tiger 0.3
	- ☐ Error dialogs
	- ☐ In-place tutorials instead of blank data
	- ☐ View animations and frames at the same time for faster browsing?
	- ☐ Multiple selections
	- ☐ Prettier UI
	- ☐ Jump to next/previous frame
	- ☐ Export perf fixes
	- ☐ Handle missing frame files (warning + offer to relocate)
	- ☐ Copy/paste hitboxes
	- ☐ Auto reload on frame edit
	- ☐ Timeline follows playback
	- ☐ Timeline follows frame selection (or double click?)
	- ☐ Timeline snapping
	- ☐ Playback speed
	- ☐ Hitbox colors
	- ☐ Default paths for NFD dialogs
	- ☐ Draw hitbox names in workbench
	- ☐ Onion skin
	- ☐ Workbench tabs?

## Tiger 1.0
	- ☐ Review all TODO
	- ☐ Compile on Rust Stable
	- ☐ Remove commands threads (keep long commands thread)
	- ☐ Get rid of failure crate
	- ☐ Document template format
	- ☐ About dialog
	- ☐ Logo
	- ☐ Itch.io or other distribution method

## Post 1.0
	- ☐ Tiger CLI
	- ☐ Sheet splitter tool
	- ☐ Anchor points (like hitbox but point)
	- ☐ Place arbitrary markers ("events") on timeline
	- ☐ Copy/paste animation or animation frame (between sheets)
