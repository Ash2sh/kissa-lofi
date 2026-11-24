# Security Policy

## Scope

Kissa is a 100% local-first desktop application with no cloud backend or telemetry. Security-relevant surfaces include:

- **WebAssembly Plugin Sandbox (`wasmi`)**: Untrusted `.wasm` / `.wat` plugins loaded by users must remain strictly sandboxed with no filesystem, network, or host memory escape.
- **Local Media Decoding**: Parsing local audio (`symphonia`), image (`image`), and video (`ffmpeg`/`ffprobe` subprocess invocation) files safely without command injection or unbounded memory allocation.
- **Share Codes**: Offline deserialization of `lofi-scene-v1:...` base64/JSON payloads.

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not** open a public issue.

Instead, use GitHub's **Private Vulnerability Reporting** feature on the repository's **Security** tab (`Security → Advisories → Report a vulnerability`), including:

- A clear description of the issue and potential impact
- Minimal reproduction steps or proof-of-concept file
- Your operating system and Kissa version

We aim to acknowledge reports within 72 hours and coordinate a fix before public disclosure.
