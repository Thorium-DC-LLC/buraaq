# Winget package — `winget install buraaq`

Moniker `buraaq` makes the short install name work after Microsoft merges the manifests.

## Packager checklist

1. Build and zip (includes `buraaq.exe` + `sysroot/`):

   ```powershell
   .\scripts\pack-winget.ps1 -Version 1.0.0
   ```

2. Upload `dist\buraaq-1.0.0-windows-x64.zip` to the GitHub Release `v1.0.0`
   (asset name must match `InstallerUrl` in the installer YAML).

3. Copy SHA256 from `dist\winget-1.0.0.sha256.txt` into
   `1.0.0/ThoriumDC.Buraaq.installer.yaml` → `InstallerSha256`.

4. Validate:

   ```powershell
   winget validate .\packaging\winget\1.0.0\
   winget install --manifest .\packaging\winget\1.0.0\
   buraaq doctor
   ```

5. Fork [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs), copy
   `packaging/winget/1.0.0/*` to:

   `manifests/t/ThoriumDC/Buraaq/1.0.0/`

6. Open a PR. After merge (often 1–3 days), anyone can run:

   ```powershell
   winget install buraaq
   ```

LLVM is declared as a dependency (`LLVM.LLVM`). Users who already have clang on PATH are fine.

## Identifier

| Field | Value |
|-------|--------|
| PackageIdentifier | `ThoriumDC.Buraaq` |
| PackageName | `Buraaq` |
| Moniker | `buraaq` → `winget install buraaq` |
