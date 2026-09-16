# Roadmap (after 1.0)

Buraaq 1.0 is the language you install and ship. What remains is hardening and ecosystem, not a second language.

## Now

| Item | Why |
|------|-----|
| Gate D 7-day fuzz | Wall-clock evidence; scripts already refuse to lie |
| Gate J on GitHub | Prove the existing workflow green |
| Guest rebuild of `llvm.bq` | Full rustc-off of `compiler-buraaq` |

## Next

| Item | Why |
|------|-----|
| Package registry `packages.buraaq.dev` | Fetch beyond path deps |
| POSIX HTTPS with OpenSSL by default | Windows WinINet already works |
| JSON DOM | Field extract is 1.0; full tree is next |
| Channels / cancellation | Spawn works; richer concurrency |
| DAP pretty printers | clang `-g` already emits symbols |

## Not the product

- A guest OS inside Ship (that is Docker; we are not)
- Lifetime annotations in source
- A second standard library with overlapping names

Bootstrap truth: [BOOTSTRAP.md](BOOTSTRAP.md). Measured ledger: [STATUS.md](STATUS.md).
