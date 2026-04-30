# BXR Worker Protocol v1 Draft

The browser runtime communicates with the machine worker through explicit commands and events.

## Commands

- `CreateMachine`
- `LoadBootArtifact`
- `Run`
- `Pause`
- `StepInstruction`
- `Snapshot`
- `Restore`
- `Reset`

## Events

- `MachineCreated`
- `Paused`
- `Running`
- `Stopped`
- `SerialOutput`
- `SnapshotReady`
- `Fault`
- `Trace`

## Rule

The main thread never executes guest instructions. It sends commands, receives events, and renders UI state.

