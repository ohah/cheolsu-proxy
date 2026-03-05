## Purpose

Restore GitHub Pages deploy workflow (deploy-pages) and simplify Cursor command rules by removing review/init-pr related commands and SSH/gh-account instructions from PR rules.

## Description

- **Deploy workflow**: Reverted `.github/workflows/deploy-docs.yml` to use `deploy-pages` with `github-pages` environment (after environment protection rules were relaxed for main).
- **Documentation**: Updated `document/README.md` to state that Pages source is GitHub Actions.
- **Cursor commands**: Removed SSH remote and gh auth switch instructions from `.cursor/commands/pr.md`. Deleted `init-pr.md`, `review.md`, and `copilot-review.md`.

## How to test

- Confirm workflow runs on push to main when `document/**` or the workflow file changes.
- Confirm GitHub Pages builds from the workflow (Settings → Pages → Source: GitHub Actions).
