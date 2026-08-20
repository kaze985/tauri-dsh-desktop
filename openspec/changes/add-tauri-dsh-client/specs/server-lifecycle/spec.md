## Purpose

Manages the full lifecycle of the locally served dsh web process: startup, readiness, termination, and failure handling.

## ADDED Requirements

### Requirement: Port precheck before startup

The client SHALL verify that the service port is free before launching dsh web, and SHALL NOT launch a second service instance when the port is already occupied.

#### Scenario: Port is occupied by another program
- **WHEN** the client starts and the service port is already in use by a process that is not this client's dsh service
- **THEN** the client reports a port-occupied error with an explanation and provides an exit action, and does not launch dsh

#### Scenario: Port is free
- **WHEN** the client starts and the service port is free
- **THEN** the client proceeds to launch the dsh service on that port

### Requirement: Service startup and readiness

The client SHALL launch the dsh web service and SHALL treat the service as ready only after it accepts connections on the service port; it SHALL NOT present the service UI before readiness is confirmed.

#### Scenario: Service becomes ready within the timeout
- **WHEN** the client launches dsh web and the port starts accepting connections within the readiness timeout
- **THEN** the window navigates to the service UI

#### Scenario: Service does not become ready
- **WHEN** the service fails to launch or the port does not accept connections within the readiness timeout
- **THEN** the client presents a startup-failure error page with a retry action and an exit action

### Requirement: Clean termination of the service process tree

When the user exits the application, the client SHALL terminate the dsh process tree it started, so that no orphaned service process keeps running or holds the service port.

#### Scenario: User exits the application
- **WHEN** the user chooses to exit the application
- **THEN** the dsh service process tree is terminated and the service port is released

### Requirement: Crash detection and recovery

The client SHALL detect when the dsh service exits unexpectedly while the application is still running and SHALL present a recovery path to the user.

#### Scenario: Service crashes while application is running
- **WHEN** the dsh service process exits unexpectedly during a session
- **THEN** the client presents a service-stopped page with a restart action and an exit action

#### Scenario: Service dies while window is hidden in tray
- **WHEN** the dsh service exits unexpectedly while the window is minimized to the tray
- **THEN** the client restores the window and presents the service-stopped page
