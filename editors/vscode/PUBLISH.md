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

After publish, users install with:

```text
ext install ThoriumDC.buraaq
```

or search **Buraaq** in the Extensions view.

## Until Marketplace is live

```powershell
code --install-extension https://github.com/ThoriumDC/buraaq/releases/latest/download/buraaq-1.0.0.vsix
```
