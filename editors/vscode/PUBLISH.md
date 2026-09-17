# Publish to VS Code Marketplace

Publisher id in `package.json`: **ThoriumDCLLC**  
Extension id users search: **Buraaq** (`ThoriumDCLLC.buraaq`)

## One-time Microsoft setup (required — cannot be automated)

1. Open [Create publisher](https://marketplace.visualstudio.com/manage/createpublisher)  
   - **Publisher ID:** `ThoriumDCLLC` (must match `package.json`)  
   - **Name:** Thorium DC, LLC  

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

Or add GitHub secret `VSCE_PAT` on `Thorium-DC-LLC/buraaq`, then run workflow **Publish VS Code extension**.

After publish, users install with:

```text
ext install ThoriumDCLLC.buraaq
```

or search **Buraaq** in the Extensions view.

## Until Marketplace is live

```powershell
code --install-extension https://github.com/Thorium-DC-LLC/buraaq/releases/latest/download/buraaq-1.0.0.vsix
```
