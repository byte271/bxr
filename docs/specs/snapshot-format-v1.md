# BXR Snapshot Format v1 Draft

Snapshots are versioned manifests plus content-addressed chunks. The format must not depend on Rust struct layout or browser object identity.

## Manifest Fields

- `format_version`
- `machine_profile`
- `parent`
- `created_by`
- `chunks`

## Required Chunks

- CPU state.
- RAM pages or RAM delta pages.
- Device state.
- Virtual time and scheduler state.
- Storage overlay references when storage exists.

## Recreated State

- TLBs.
- Decoded instruction blocks.
- Trace/JIT caches.
- Renderer textures.
- OPFS access handles.
- Browser worker instances.

## First Implementation Rule

The first implementation may keep manifests in memory as typed Rust values, but the public format must remain independent of in-process memory layout.

