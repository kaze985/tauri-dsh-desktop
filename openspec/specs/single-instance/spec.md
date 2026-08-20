# single-instance Specification

## Purpose

Guarantees that only one instance of the client runs on the machine, preventing duplicate dsh services and configuration conflicts.

## Requirements

### Requirement: Single running instance

The client SHALL allow only one running instance per machine; a subsequent launch SHALL focus the existing window and exit without starting a second dsh service.

#### Scenario: Second launch while app is running
- **WHEN** the user launches the application while an instance is already running
- **THEN** the new process exits, the existing window becomes visible and focused, and no second dsh service is started

#### Scenario: Second launch while window is hidden in tray
- **WHEN** the user launches the application while the existing instance is minimized to the tray
- **THEN** the existing window is restored and focused, and the new process exits
