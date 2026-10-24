# Contributing to Mstudio

Thank you for helping improve Mstudio. Bug reports, documentation, translations and focused fixes are welcome. Discuss larger behavior changes in an issue before starting implementation.

## Report a bug

Open a [GitHub issue](https://github.com/Yaozy-C/mstudio/issues) with:

- Your OS version and the affected commit or build.
- The steps to reproduce, expected behavior and actual behavior.
- Sanitized diagnostics and, when useful, a screenshot or minimal example using synthetic media.

Do not include API keys, application databases, private conversations or personal media. Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## Set up and submit a change

1. Review [LICENSE](LICENSE) and agree on the contribution scope and terms with the maintainer before preparing code changes; then create a branch for the agreed change.
2. Follow the [README quick start](README.md#quick-start). The detailed [development guide](docs/development.md) is currently in Chinese.
3. Keep the change focused on one problem. Add a regression test when it meaningfully demonstrates a behavior fix.
4. Run `sh scripts/check.sh`. Changes to Skills also require `python3 scripts/check_skills.py`.
5. If your change affects native preview, run the relevant GES media regression checks from the development guide and describe the platform you tested manually.
6. Open a pull request explaining the problem, the resulting behavior, validation and known limitations. State explicitly when a check was not run.

The full check suite needs the native GES dependencies and permission to listen on loopback for local HTTP tests. It does not require external model credentials.

## Code and repository conventions

- Follow the existing Rust and Prettier formatting. Avoid unrelated refactoring.
- Split files by responsibility when it improves readability; avoid wrappers and extra modules added only to satisfy a line count.
- Keep lockfiles committed and update them when changing dependencies.
- Do not commit compiled applications, native runtime bundles, credentials, databases, real chat logs, personal media, generated production media, design explorations or review screenshots. The curated public screenshots in `assets/screenshots/` use original demo content and are documentation assets.
- Bundled effect preview videos are limited to `frontend/public/effects/sources.json`: each entry records its source, exact byte count and SHA-256. The release check allows only matching MP4s up to 3 MiB; other media restrictions remain in place. Source metadata does not establish redistribution rights.
- Use synthetic data for tests. Keep error output free of credentials and private project content.
- New interface text must use the existing localization system, with English and Simplified Chinese translations. Never translate user-authored content implicitly.

## Interface changes

Follow the existing Frame/Cut visual language: warm neutral surfaces, clear typography, restrained dividers and crop-red selection accents. Reuse the shared components and styles rather than creating local variants.

Use the existing Phosphor icons according to their function. Do not use sparkle/starburst, magic-wand or robot-avatar imagery, including custom SVGs and emoji. Ordinary inline actions should use the shared `ActionButton`; inspector panels should use the shared `DockPanel` structure.

Use `IconButton` for icon-only actions (a required label supplies the accessible name and tooltip), `ActionButton` for inline actions, and `AsyncButton` for pending form submissions. Pass `busy` to prevent repeat submission; reserve pending-label width. Use `aria-pressed` for toggles, `aria-invalid` for validation, and native `disabled` for unavailable actions.

Use Radix for menus, selects and dialogs. Object overflow menus use `ObjectActions`; context menus use `ObjectMenu`, sharing `ObjectAction` including its `danger` and `disabled` semantics. Apply `studio-menu` to other dropdowns. Do not implement local keyboard-navigation or outside-click handlers. Native dialog menus must portal inside the dialog top layer.

Keep control geometry in its component stylesheet and shared feedback in `styles/control-states.css`, using the existing color and motion tokens. Default, hover, pressed, focus-visible, selected/open, disabled, busy and invalid states must remain distinct. Portal content can itself be the `.radix-themes` root. Do not add broad transforms to buttons that represent draggable canvas or timeline objects. Use `/tests/control-states.html` in Vite to inspect both languages, keyboard dismissal, disabled-item skipping and native-dialog menus.

Verify the affected workspace, empty and populated states, focus behavior and readable English/Chinese labels. Browser checks do not validate native dialogs, media playback or real generation services.

## Licensing and community

The current project licensing notice is [LICENSE](LICENSE); previously published MIT material is covered by [LICENSE-MIT-LEGACY](LICENSE-MIT-LEGACY). Agree on contribution terms with the maintainer before submitting new material. Preserve the source and applicable license of third-party content. Keep discussions respectful and focused on the code and behavior.
