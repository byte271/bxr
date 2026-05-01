# BXR Worker Protocol v1 Draft

The browser runtime communicates with the machine worker through explicit commands and events.

## Commands

- `CreateMachine`
- `Step`
- `RunDemo`
- `Pause`
- `Snapshot`
- `Restore`

Future commands:

- `LoadBootArtifact`
- `Run`
- `StepInstruction`
- `Reset`

## Events

- `MachineCreated`
- `Paused`
- `Running`
- `Stopped`
- `SnapshotReady`
- `Fault`
- `SerialState`
- `DebugState`

Future events:

- `SerialOutput`
- `Trace`

## Rule

The main thread never executes guest instructions. It sends commands, receives events, and renders UI state.
