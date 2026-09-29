# Contributing to Mstudio

Thank you for helping improve Mstudio. Bug reports, documentation, translations and focused fixes are welcome. Discuss larger behavior changes in an issue before starting implementation.

## Report a bug

Open a [GitHub issue](https://github.com/Yaozy-C/mstudio/issues) with:

- Your OS version and the affected commit or build.
- The steps to reproduce, expected behavior and actual behavior.
- Sanitized diagnostics and, when useful, a screenshot or minimal example using synthetic media.

Do not include API keys, application databases, private conversations or personal media. Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## Set up and submit a change

1. Fork the repository and create a branch for your change.
2. Follow the [README quick start](README.md#quick-start). The detailed [development guide](docs/development.md) is currently in Chinese.
3. Keep the change focused on one problem. Add a regression test when it meaningfully demonstrates a behavior fix.
4. Run `sh scripts/check.sh`. Changes to Skills also require `python3 scripts/check_skills.py`.
5. If your change affects native preview, run the relevant GES media regression checks from the development guide and describe the platform you tested manually.
6. Open a pull request explaining the problem, the resulting behavior, validation and known limitations. State explicitly when a check was not run.

The full check suite needs the native GES dependencies and permission to listen on loopback for local HTTP tests. It does not require external model credentials.

## Code and repository conventions

- Follow the existing Rust and Prettier formatting. Avoid unrelated refactoring.
- Keep each maintained source, test, rule or configuration file at or below **300 lines**. Split by responsibility; do not add exemptions. Dependency lockfiles and third-party generated files are managed by their tools.
- Keep lockfiles committed and update them when changing dependencies.
- Do not commit compiled applications, native runtime bundles, credentials, databases, real chat logs, personal media, generated production media, design explorations or review screenshots. The curated public screenshots in `assets/screenshots/` use original demo content and are documentation assets.
- Use synthetic data for tests. Keep error output free of credentials and private project content.
- New interface text must use the existing localization system, with English and Simplified Chinese translations. Never translate user-authored content implicitly.

## Interface changes

Follow the existing Frame/Cut visual language: warm neutral surfaces, clear typography, restrained dividers and crop-red selection accents. Reuse the shared components and styles rather than creating local variants.

Use the existing Phosphor icons according to their function. Do not use sparkle/starburst, magic-wand or robot-avatar imagery, including custom SVGs and emoji. Ordinary inline actions should use the shared `ActionButton`; inspector panels should use the shared `DockPanel` structure.

Verify the affected workspace, empty and populated states, focus behavior and readable English/Chinese labels. Browser checks do not validate native dialogs, media playback or real generation services.

## Licensing and community

Contributions are submitted under the project's [MIT License](LICENSE). Preserve the source and applicable license of third-party content. Keep discussions respectful and focused on the code and behavior.
