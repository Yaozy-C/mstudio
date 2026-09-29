# Security policy

## Supported code

Mstudio is in active development. Security fixes target the latest code on the default branch; older versions are not currently maintained separately.

## Report a vulnerability

Use the repository's **Security → Report a vulnerability** entry on GitHub to report privately. If that entry is unavailable, contact the maintainer to arrange a private channel before sharing sensitive details. Do not post credentials, exploit details or private project data in public issues.

Include the affected commit, operating system, reproduction steps, impact and a minimal example without private data. Ordinary bugs can be reported through [GitHub issues](https://github.com/Yaozy-C/mstudio/issues).

## Current security considerations

Model API keys are stored in the local SQLite database. They are not currently protected by the system keychain. Do not share the application database or entire application data directory; protect your own backups.

Model endpoints should only be configured by trusted users. Requests to online models transmit prompts and selected references to the configured provider. Local project storage does not imply that all AI processing happens locally.
