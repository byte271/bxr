# Security

BXR executes untrusted guest machine code inside the browser runtime. The current prototype is not a hardened sandbox product.

## Supported Security Boundary

The intended boundary is the browser security model plus WebAssembly memory safety and worker isolation. The runtime does not provide native virtualization isolation.

## Current Limitations

- Machine package import/export is not finalized.
- Network devices are not implemented.
- Persistent disk images are not implemented.
- Snapshot bundle validation is still experimental.
- Denial-of-service through CPU, memory, or storage exhaustion is not fully mitigated.

## Reporting Issues

Please report security issues privately to the repository owner before public disclosure. Include reproduction steps, browser version, operating system, and whether the issue requires a malicious guest, malicious package, or malicious hosting environment.
