# Security policy

Buraaq is a systems language and a native deploy toolchain. Use it on systems you own or are authorized to test. **Do not abuse.**

## Legitimate use

Allowed:

- Writing, compiling, and shipping your own Buraaq programs
- Testing Keel, Dock, Ship, and Land on hosts you control or have written permission to use
- Reporting crashes, memory-safety bugs, and integrity failures to the maintainers
- Security research that stays within authorized scope

Not allowed:

- Scanning, flooding, or breaking into Dock (`:7422`), Keel (`:8080` / `:8443`), or any host you do not operate
- Using Buraaq to steal data, hide malware, or attack third parties
- Sharing or committing production secrets (database URLs, dock tokens, API keys)
- Treating Dock as a multi-tenant sandbox for untrusted code

Dock isolation is a **private directory and a child process**, not a VM. Hash checks stop tampered `.bur` files. They do not replace authorization.

## Cooperation with agencies

Buraaq administration **cooperates with lawful requests** from law-enforcement and government agencies. We do not provide a platform for crime, and we do not obstruct legitimate investigations.

If you are an agency with a lawful request, contact the founding author:

- Asim — [linkedin.com/in/mdasimaslam](https://linkedin.com/in/mdasimaslam)
- Site: [buraaq.dev/security](https://buraaq.dev/security)

We respond in good faith. We do not publish exploit recipes. We do not assist unauthorized access.

## Reporting a vulnerability

**Do not** open public issues for exploitable compiler or stdlib bugs.

Include Buraaq version or commit, a `.bq` reproducer, and impact (crash, code execution, data leak). We aim to acknowledge within 72 hours. Coordinated disclosure is preferred. Credit is given unless you ask for anonymity.

## Supported versions

| Version | Supported |
|---------|-----------|
| 1.0 | Security patches for the current release |
| Pre-1.0 snapshots | Best-effort on main |

## Scope

In scope:

- Compiler crashes on valid or invalid **source** (denial of service in IDE/CI)
- Memory-safety bugs in **runtime** (`stdlib/runtime/`)
- Package or ship parsing that allows path traversal or unexpected code execution
- Keel/Dock auth bypass on a default install

Out of scope:

- User `unsafe` that violates documented rules
- Attacks against hosts you are not authorized to test
- Third-party services (Postgres providers, clouds) outside the Buraaq runtime

## Hardening

- Compiler pipeline fuzz (mutated programs + random bytes); Gate D is a 7-day wall-clock run
- Invalid programs must produce diagnostics, not process abort (except OOM)
- Ships are SHA-256 hashed; path `..` is rejected; Dock mutations need a bearer token
- App TLS is independent of Dock HTTP. Keep `:7422` off the public internet unless you intend remote `buraaq ship`

## Secrets

Never commit `BURAAQ_DATABASE_URL`, dock tokens, or API keys. Put them on the **host** environment. Rotate any credential that has appeared in a chat, a log, or a screenshot.
