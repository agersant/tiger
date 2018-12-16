
# Roadmap

## Tiger 0.1
- [x] Don't store absolute paths in tiger files
- [x] Show frame names in timeline
- [x] Solution(s) to edit/order/insert frames in timeline
- [x] Add, position, tag, delete hitboxes
- [x] Full compat with crystal sheets:
	- [x] Hitboxes
	- [x] Sheet path in export
	- [x] Top left coords available in template
- [x] Tiger backwards compat
- [x] Release pipeline
- [x] No placeholder menu options
- [x] Export (using last known settings)
- [x] Draw frame being dragged even during animation
- [x] Draw hitboxes during animation
- [x] Animation renames
- [x] Allow user to choose what directory paths are relative to during export
- [x] Fix bug where export window shows weird absolute + relative concatenated paths
- [x] Fix bug where pressing delete while renaming an animation(/hitbox) deletes it
- [x] Fix bug where renaming an animation(/hitbox) unselects and unedits it
- [x] Fix bug where animation frame duration drag shows insert markers
- [x] Fix bug where animation frames can be reorderer by dragging timeline
- [x] Fix bugs when manipulating extremely short animation frames

## Tiger 0.2
- [ ] Unsaved file marker and warnings
- [ ] Undo/Redo
- [ ] Keyboard shortcuts for playback controls
- [ ] Keyboard shortcuts menu entries
- [ ] Keyboard shortcuts for list navigation
- [ ] Loading spinners
- [x] Begin editing animation after creating it
- [x] Select hitbox after creating it
- [ ] Duplicate animation / animation frame (within same sheet)
- [ ] Grid
- [ ] Drag and drop frames to workbench
- [ ] Grid snapping
- [ ] Content of selection window when selecting animation frame
- [ ] Content of selection window when selecting hitbox
- [ ] In selection window, keep origin centered to preview turnarounds
- [ ] When moving animation frame or hitbox, hold shift to move only on one axis
- [ ] When resizing hitbox, hold shift to make square (or preserve aspect ratio?)
- [ ] Content window and workbench should say what the current workbench item is
- [x] Sort content panel entries by name
- [x] Sort hitbox panel entries by name
- [x] Dont draw origin when editing frame
- [ ] Use rect and point structs consistently instead of tuples everywhere
- [ ] Fix bug where origin is not consistent within one animation in selection window (is ok in workbench)
- [x] Fix bug where frame name can go outside frame bound in timeline
- [ ] Fix bug where reordering animation frames changes selected animation frame
- [ ] Fix bug where a console window opens alongside Tiger on Windows

## Tiger 0.3
- [ ] Error dialogs
- [ ] In-place tutorials instead of blank data
- [ ] View animations and frames at the same time for faster browsing?
- [ ] Multiple selections
- [ ] Prettier UI
- [ ] Jump to next/previous frame
- [ ] Export perf fixes
- [ ] Handle missing frame files (warning + offer to relocate)
- [ ] Copy/paste hitboxes
- [ ] Auto reload on frame edit
- [ ] Timeline follows playback
- [ ] Timeline follows frame selection (or double click?)
- [ ] Timeline snapping
- [ ] Playback speed
- [ ] Hitbox colors
- [ ] Default paths for NFD dialogs
- [ ] Draw hitbox names in workbench
- [ ] Onion skin
- [ ] Workbench tabs?
- [ ] Editing hitboxes while animation is in workbench?

## Tiger 1.0
- [ ] Review all TODO
- [ ] Provide export templates for some common formats (TBD)
- [ ] Compile on Rust Stable
- [ ] Remove commands threads (keep long commands thread)
- [ ] Get rid of failure crate
- [ ] Document template format
- [ ] About dialog
- [ ] Logo
- [ ] Itch.io or other distribution method

## Post 1.0
- [ ] Tiger CLI
- [ ] Sheet splitter tool
- [ ] Anchor points (like hitbox but point)
- [ ] Place arbitrary markers ("events") on timeline
- [ ] Copy/paste animation or animation frame (between sheets)
