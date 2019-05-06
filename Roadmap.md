
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
- [x] Unsaved file marker
- [x] Offer to save on exit if needed
- [x] Undo/Redo
- [x] Allow interactions while file dialogs are open or I/O is in progress
- [x] Keyboard shortcuts for playback controls
- [x] Keyboard shortcuts menu entries
- [x] Keyboard shortcuts for moving hitbox / animation frame
- [x] Keyboard shortcuts for list navigation
- [x] Loading spinners
- [x] Begin editing animation after creating it
- [x] Automatically select hitbox after creating it
- [x] Select hitbox when clicking it
- [x] Select animation frame when clicking it
- [x] Selected hitbox should have handles for resizing instead of using invisible buttons along borders
- [x] When creating an animation, automatically select it
- [x] Grid
- [x] Drag and drop frames to workbench
- ~~[ ] Grid snapping?~~
- [x] Content of selection window when selecting animation frame
- [x] Content of selection window when selecting hitbox
- [x] In selection window, keep origin centered to preview turnarounds
- [x] When moving animation frame or hitbox, hold shift to move only on one axis
- [x] When resizing hitbox, hold shift to preserve aspect ratio
- [x] Workbench indicates what the current workbench item is
- [x] Sort content panel entries by name
- [x] Sort hitbox panel entries by name
- [x] Dont draw origin when editing frame
- [x] Use rect and point structs consistently instead of tuples everywhere
- [x] Fix bug where origin is not consistent within one animation in selection window (is ok in workbench)
- [x] Fix bug where frame name can go outside frame bound in timeline
- [x] Fix bug where reordering animation frames changes selected animation frame
- [x] Fix bug where a console window opens alongside Tiger on Windows
- [x] Workbench should illustrate selected hitbox or animation frame (w/ borders)
- [x] Clicking blank space within the workbench gets rid of the current selection
- [x] Ctrl+Space to center workbench
- [x] Fix issue where hitboxes are not created precisely where the mouse is clicked because we dont create until the mouse is dragging.
- [x] Pass in mouse drag deltas to drag/resize logic instead of mouse positions. See GetMouseDragDelta in imgui
- [x] Handle scenario when using "Save as" onto a file that is already open

## Tiger 0.3
- [x] Cap undo history at 100 entries
- [x] Offer to save when closing individual documents
- [ ] Error dialogs
- [ ] Handle save errors while performing a save on exit
- [ ] Add option to hide hitboxes while looking at animations in workbench
- [ ] Multiple selections
- [x] Jump to next/previous frame
- [ ] Export perf fixes
- [ ] Handle missing frame files (warning + offer to relocate)
- [ ] Copy/paste hitboxes
- [x] Auto reload images on frame edit
- [ ] Timeline scrolling follows playback
- [ ] Timeline scrolling follows frame selection (or double click?)
- [ ] Time snapping of animation frames
- [ ] Playback speed controls
- [ ] Hitbox colors
- [ ] Default paths for NFD dialogs
- [ ] Draw hitbox names in workbench
- [ ] Onion skin?
- [ ] Editing hitboxes while animation is in workbench? Double click to edit frame?
- [ ] Duplicate animation / animation frame (within same sheet)

## Tiger 0.4
- [ ] Review TODO dpi
- [ ] Workbench tabs?
- [ ] Text filtering of frames/animations
- [ ] View animations and frames at the same time for faster browsing?
- [ ] Content panel shows previews of frames and animations? (list view? grid view?)
- [ ] Better rename UX
- [ ] Right click menu to rename/delete item
- [ ] Document tabs (imgui 1.67+)
- [ ] Fix issue where O key gets stuck after using Ctrl+O shortcut (https://github.com/Gekkio/imgui-rs/pull/215)
- [ ] In-place tutorials instead of blank data
- [ ] Prettier UI and review TODO.style
- [ ] Unit-test all the things

## Tiger 1.0
- [ ] Review all TODO
- [ ] Provide export templates for some common formats (TBD)
- [ ] Compile on Rust Stable
- [x] Remove commands threads (keep long commands thread)
- [ ] Get rid of failure crate
- [ ] Document template format
- [ ] About dialog
- [ ] Logo
- [ ] Itch.io or other distribution method

## Post 1.0
- [ ] Tiger CLI
- [ ] Open Recent
- [ ] Sheet splitter tool
- [ ] Import animation data from other software (TBD)
- [ ] Anchor points (like hitbox but point)
- [ ] Place arbitrary markers ("events") on timeline
- [ ] Copy/paste animation or animation frame (between sheets)
- [ ] Projects
