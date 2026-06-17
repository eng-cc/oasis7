# Xiaohongshu Social Packages

This directory stores Xiaohongshu content packages. Each package keeps copy/governance docs, editable visual sources, and exported publishing assets together.

## Layout

```text
site/social/xiaohongshu/
  <post-slug>/
    README.md
    *-post-pack-*.md / *-carousel-pack-*.md
    cover.html / carousel.html
    assets/
    exports/
  _shared/
```

## Rules

- Put new Xiaohongshu content in `site/social/xiaohongshu/<post-slug>/` rather than the old flat `site/social/` directory.
- Use package `README.md` as the manifest for publish-ready exports, preview-only images, source assets, and related copy docs.
- Keep channel-wide SOP in `doc/readme/governance/readme-xiaohongshu-liveops-runbook-2026-03-23.md`.
- Use `_shared/` only for cross-package templates, scripts, or shared visual primitives.
