# tray-and-close Specification

## Purpose

Defines tray presence and the window-close semantics: exit versus minimize-to-tray, with a persistent user choice.

## Requirements

### Requirement: Close dialog with exit and tray options

When the user closes the main window, the client SHALL ask whether to exit the application or minimize the window to the tray, unless the user has previously saved a choice.

#### Scenario: Close without remembered choice
- **WHEN** the user closes the window and no close choice has been remembered
- **THEN** the client presents a dialog offering exit or minimize-to-tray, and applies the chosen action

#### Scenario: Close with remembered choice
- **WHEN** the user closes the window and a close choice has been remembered
- **THEN** the client applies the remembered action without showing the dialog

### Requirement: Tray presence and menu

The client SHALL provide a tray icon with a menu containing at least a show-window action and an exit action, and minimizing to the tray SHALL hide the window while the dsh service keeps running.

#### Scenario: Restore window from tray
- **WHEN** the user activates show-window from the tray menu while the window is hidden
- **THEN** the window becomes visible and focused

#### Scenario: Exit from tray
- **WHEN** the user activates exit from the tray menu
- **THEN** the application exits and terminates the dsh service process tree

### Requirement: Persistent close choice

The dialog SHALL offer a remember option that persists the user's choice across application restarts, and the tray menu SHALL provide a way to reset it.

#### Scenario: Remember and reset
- **WHEN** the user selects the remember option in the close dialog, the choice persists across restarts; **WHEN** the user later resets it from the tray menu
- **THEN** the next window close shows the dialog again
