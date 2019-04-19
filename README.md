[![Build Status](https://travis-ci.org/agersant/tiger.svg?branch=master)](https://travis-ci.org/agersant/tiger)

Tiger is a graphical tool for generating spritesheets and metadata about the animation and hitboxes they contain.

![Tiger](res/readme/screenshot-0.1.0.png?raw=true "Tiger")

# Key Features

- Timeline-editing for authoring animations
- Easy to add and position hitboxes
- Support for custom formats when exporting metadata
- Generated texture atlas for use in-engine
- Free and open-source with a permissive license

⚠️ This project is under development. It is already usable and is generating spritesheets for [Project Crystal](https://github.com/agersant/crystal). However, you should come back in a few months if you are looking for a polished experience!

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
2. Install the Rust compiler by executing `curl https://sh.rustup.rs -sSf | sh` or using an [alternative method](https://www.rust-lang.org/en-US/install.html)

#### Tiger installation
1. Download the [latest release]((https://github.com/agersant/tiger/releases/latest)) of Tiger (you want the .tar.gz file)
2. Extract the archive in a directory and open a terminal in that directory
3. Execute `make install` (this may take several minutes)

This installation process puts the Tiger executable in `~/.local/bin/tiger`.

If you want to uninstall Tiger, execute `make uninstall` from the extracted archive's directory. This will simply delete the files created by the install process.

# Roadmap

See [here](Roadmap.md).