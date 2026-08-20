## Purpose

Ensures the dsh CLI is installed and keeps its version up to date through user-confirmed upgrades.

## ADDED Requirements

### Requirement: dsh installation guarantee

Before starting the service, the client SHALL verify that the dsh CLI is available on the machine, and SHALL guide the user through installation when it is missing.

#### Scenario: dsh is installed
- **WHEN** the dsh CLI is available on the machine
- **THEN** the client proceeds with the startup sequence

#### Scenario: dsh is missing
- **WHEN** the dsh CLI is not available on the machine
- **THEN** the client presents a setup page that explains the requirement and performs or guides the installation before proceeding

### Requirement: Update check against the registry

The client SHALL compare the installed dsh version with the latest version published to the npm registry, SHALL notify the user only when a newer version exists, and SHALL NOT block startup when the check fails.

#### Scenario: Newer version available
- **WHEN** the installed dsh version is older than the registry latest
- **THEN** the client notifies the user that an update is available

#### Scenario: Registry unreachable
- **WHEN** the registry check fails due to network problems
- **THEN** the client silently skips the check and starts normally with the installed version

### Requirement: User-confirmed upgrade

An upgrade SHALL only run after explicit user confirmation, and a successful upgrade SHALL be followed by a restart of the dsh service so the new version takes effect.

#### Scenario: User confirms an upgrade
- **WHEN** the user confirms an available update
- **THEN** the client performs the upgrade, then restarts the dsh service, and the service runs the new version

#### Scenario: User declines an upgrade
- **WHEN** the user declines an available update
- **THEN** the client keeps the installed version and starts the service normally
