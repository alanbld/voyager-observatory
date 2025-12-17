# pm_encoder - Pending Tasks

## Releases

- [ ] **PENDING:** Push v1.7.0 tag and publish GitHub Release
  - Tag `v1.7.0` is created locally
  - Release notes ready at `docs/releases/v1.7.0_PENDING.md`
  - Run when GitHub access restored:
    ```bash
    gh release create v1.7.0 \
      --title "v1.7.0 - The Intelligence Update" \
      --notes-file docs/releases/v1.7.0_PENDING.md
    ```

## Rust Parity

- [x] Rust v0.6.0 - Port Priority Groups ✅ (2025-12-17)
- [ ] Rust v0.7.0 - Port Token Budgeting
- [ ] Rust v0.8.0 - Port Budget Strategies

## Future Features

- [ ] Protocol Adapters (Claude, GPT, Gemini output formats)
- [ ] Interactive mode for budget tuning
