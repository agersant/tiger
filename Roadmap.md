# Roadmap

## For v0.1
	✓ Don't store absolute paths in tiger files
	✓ Show frame names in timeline
	· Solution(s) to edit/order/insert frames in timeline
	✓ Add, position, tag, delete hitboxes
	· Full compat with crystal sheets:
		✓ Hitboxes
		✓ Sheet path in export
		✓ Top left coords available in template
	✓ Tiger backwards compat
	✓ Release pipeline
	✓ No placeholder menu options
	✓ Export (using last known settings)
	✓ Draw frame being dragged even during animation
	✓ Draw hitboxes during animation
	✓ Animation renames

	✓ Fix bug where pressing delete while renaming an animation(/hitbox) deletes it
	✓ Fix bug where renaming an animation(/hitbox) unselects and unedits it

## For v0.2
	· Unsaved file marker and warnings
	· Undo/Redo
	· Keyboard shortcuts for playback controls
	· Keyboard shortcuts menu entries
	· Keyboard shortcuts for list navigation
	· Loading spinners
	· Duplicate animation / animation frame (within same sheet)
	· Grid
	· Drag and drop frames to workbench
	· Grid snapping
	· Content of selection window when selecting animation frame
	· Content of selection window when selecting hitbox
	· In selection window, keep origin centered to preview turnarounds
	· When moving animation frame or hitbox, hold shift to move only on one axis
	· When resizing hitbox, hold shift to make square (or preserve aspect ratio?)
	· Sort content panel entries by name
	· Sort hitbox panel entries by name

	· Fix bug where origin is not consistent within one animation in selection window (is ok in workbench)

## For v0.3
	· Error dialogs
	· In-place tutorials instead of blank data
	· View animations and frames at the same time for faster browsing?
	· Multiple selections
	· Prettier UI
	· Jump to next/previous frame
	· Export perf fixes
	· Handle missing frame files (warning + offer to relocate)
	· Auto reload on frame edit
	· Timeline follows playback
	· Timeline follows frame selection (or double click?)
	· Timeline snapping
	· Playback speed
	· Hitbox colors
	· Allow user to choose what directory paths are relative to during export
	· Draw hitbox names in workbench
	· Onion skin

## For v1.0
	· Review all TODO
	· Compile on Rust Stable
	· Document template format
	· About dialog
	· Logo
	· Itch.io or other distribution method

## Future work
	· Tiger CLI
	· Sheet splitter tool
	· Copy/paste animation or animation frame (between sheets)
