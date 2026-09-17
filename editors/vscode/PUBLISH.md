# Publish to VS Code Marketplace

Publisher id in `package.json`: **ThoriumDC**  
Extension id users search: **Buraaq** (`ThoriumDC.buraaq`)

## One-time Microsoft setup (required — cannot be automated)

1. Open [Create publisher](https://marketplace.visualstudio.com/manage/createpublisher)  
   - **Publisher ID:** `ThoriumDC` (must match `package.json`)  
   - **Name:** Thorium DC  

2. Create an Azure DevOps PAT:  
   [https://dev.azure.com](https://dev.azure.com) → User settings → Personal access tokens → New  
   - Organization: **All accessible organizations**  
   - Scopes: **Marketplace → Manage**  

3. Either:

```powershell
cd editors\vscode
$env:VSCE_PAT = "<paste-pat>"
npx vsce publish
```

Or add GitHub secret `VSCE_PAT` on `ThoriumDC/buraaq`, then run workflow **Publish VS Code extension**.

Published listing: [marketplace.visualstudio.com/items?itemName=ThoriumDC.buraaq](https://marketplace.visualstudio.com/items?itemName=ThoriumDC.buraaq)

Users install with:

```text
ext install ThoriumDC.buraaq
```

or search **Buraaq** in the Extensions view. Offline/CI fallback: the Release VSIX.

## Republish

Bump `version` in `package.json`, then `npx vsce publish` (or the GitHub Actions workflow) with a fresh `VSCE_PAT`.
