## Purpose

Defines the local shell window that wraps the remote dsh service UI: its startup sequence, loading state, error states, and navigation.

## ADDED Requirements

### Requirement: Local shell pages independent of the service

The client SHALL render loading and error pages from local bundled assets, so they remain fully functional when the dsh service is not running.

#### Scenario: Service is down during startup
- **WHEN** the client starts and the dsh service cannot be reached
- **THEN** the loading and error pages still render correctly with all actions available

### Requirement: Startup navigation sequence

The client SHALL first show a loading page, SHALL keep it visible until the service is confirmed ready, and SHALL then navigate the window to the service UI.

#### Scenario: Normal startup
- **WHEN** the user launches the application and the service becomes ready
- **THEN** the window shows the loading page first and then navigates to the service UI

### Requirement: Error presentation with recovery actions

Every startup failure mode SHALL present a human-readable cause and recovery actions.

#### Scenario: Startup failure of any kind
- **WHEN** startup fails due to port occupation, spawn failure, environment problems, or readiness timeout
- **THEN** the client presents an error page that states the failure cause and offers an appropriate recovery action (retry or exit)

### Requirement: Retry runs the full startup sequence

The retry action SHALL re-run the complete startup sequence from the port precheck onward.

#### Scenario: User retries after a failure
- **WHEN** the user activates retry on an error page
- **THEN** the client re-runs the startup sequence starting from the port precheck, and navigates to the service UI only if the service becomes ready
