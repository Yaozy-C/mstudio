# Mstudio

**A Vibe Video Studio · From mind to motion.**

**English** | [简体中文](README.zh-CN.md)

An open-source desktop video studio for working with AI, from the first idea to the final cut. Write a script, organize references and shots, generate media with your own models, and edit and export in one local workspace.

We call this **Vibe Video**: talk through an idea, see what it looks like, and refine it together. The script, production canvas and timeline keep the work editable at every step.

[Quick start](#quick-start) · [Features](#features) · [Development](#development) · [Contributing](CONTRIBUTING.md) · [Report a bug](https://github.com/Yaozy-C/mstudio/issues)

![Mstudio in English: a portrait film preview beside project media, with three clips on the timeline](assets/screenshots/timeline.jpg)

*Actual macOS desktop capture. The Daybreak project uses original demo artwork prepared for this documentation.*

> **Development preview.** macOS is the primary development and validation platform. Windows packaging and device validation are incomplete; Linux native desktop preview is not supported. Build from source using the steps below.

## Features

- **Script to shots.** Write visuals, dialogue, on-screen text and sound as separate fields. Set paragraph timing and keep shots linked to their source script.
- **A canvas for production.** Arrange references and media, inspect storyboard frames, and keep alternatives together before deciding what belongs in the film.
- **Your models, your workflow.** Configure chat and media models separately. Generate and revise images or videos with the references and controls supported by each provider.
- **Specialist Agents.** Configure a producer, writer, shot director, storyboard artist, editor and other roles with editable instructions, Skills and tool permissions. Reference specific project items in conversation.
- **An editable timeline.** Arrange multiple tracks; split, trim and retime clips; detach audio; adjust color and transitions; add captions and local voiceover; preview and export MP4.
- **Local project storage.** Automatic saving, project memory, a reusable media library and storage migration. The interface supports English and Simplified Chinese.

Manual media import, editing and local export do not require an AI account.

### Write the story and its sound

![English script workspace showing visuals, on-screen text, narration and sound for a demo scene](assets/screenshots/script.jpg)

### Keep references and media in view

![English production canvas with three original landscape illustrations arranged side by side](assets/screenshots/canvas.jpg)

## Quick start

### Build and run on macOS

Install Xcode Command Line Tools, [Rust via rustup](https://rustup.rs/), [Bun](https://bun.sh/) and [Homebrew](https://brew.sh/). The repository pins its Rust version in [`rust-toolchain.toml`](rust-toolchain.toml). Python 3.12 or newer is required.

```sh
git clone https://github.com/Yaozy-C/mstudio.git
cd mstudio

brew install python pkgconf ffmpeg gstreamer
(cd frontend && bun install --frozen-lockfile)
python3 scripts/bundle-ges.py
sh scripts/dev.sh
```

The development script starts the frontend and the Tauri desktop application. Use the desktop application for file imports, native playback, generation and export; the browser preview alone does not provide the complete workflow.

To build a local app bundle:

```sh
sh scripts/bundle.sh
open Mstudio.app
```

This creates `Mstudio.app` in the repository root. See the [development guide](docs/development.md) for native dependencies, media regression checks and platform limitations.

### Make your first film

1. Open **General → Language** to choose English or 简体中文.
2. Create a project. Import your own media, or start by writing a script.
3. For AI assistance, open **Models**, add a service connection and configure a chat model. Add image or video models when you need generation.
4. In chat, use `@` to reference project items and `$` to choose a specialist. For example: “Help me outline a 15-second landscape film. Keep the pace quiet and leave room for natural sound.”
5. Arrange and inspect media in **Production canvas**, then edit in **Film**. Review picture and sound before exporting.

Remote AI services require your own account or API key and may charge for usage. Model inputs, generation controls and availability depend on the configured provider.

## Models and Agents

Chat connections support OpenAI-compatible APIs, Responses, Gemini and native Claude protocols, including compatible local services. Media generation uses separate adapters and model configuration; support for a chat protocol does not automatically imply image or video generation support.

Agents share project context and can use permitted tools to update scripts, shots and edits. Their instructions and assigned Skills are configurable. Child conversations distinguish independent creation, inheritance of completed parent history, and continuation of an existing session. See the [architecture guide](docs/architecture.md) for the Rust host and execution pipeline.

## Data and privacy

- Projects and media are stored locally. On macOS, the default application data directory is `~/Library/Application Support/local.mstudio.canvas/`; media may be stored elsewhere if you move it in **General → Storage**.
- Online model requests send prompts and selected references to the configured provider. Local storage does not mean all AI processing happens on your computer.
- API credentials currently reside in the local SQLite database and are **not protected by the system keychain**. Do not share your database, full application data directory or credential-bearing logs.
- Autosave is not versioned backup. Quit the app before backing up its application data and any separately configured media directory.

Report vulnerabilities privately using the instructions in [SECURITY.md](SECURITY.md).

## Current limitations

- AI output needs human review for visual consistency, motion, timing and sound.
- Transitions currently target adjacent, opaque, full-frame base-layer visuals. They do not provide optical flow, occlusion masks or audio crossfades.
- Color controls are not a complete professional color-management workflow.
- Native playback, file dialogs and real model integrations still require desktop validation. Passing automated checks is not a substitute for watching the finished film.

## Development

Mstudio uses **React + TypeScript** for the interface, **Tauri + Rust** for desktop services, **SQLite** for persistence, **GStreamer Editing Services (GES)** for native preview and **FFmpeg** for media processing and export.

```text
frontend/       React interface, canvas, timeline and Agent configuration
desktop/        Tauri host, persistence, Agent runtime and model adapters
src/            Shared Rust editing and media logic
skills/         Bundled creative methods and role guidance
scripts/        Development, validation and macOS packaging
```

Run the required checks after preparing the native dependencies:

```sh
sh scripts/check.sh
```

This checks formatting, source release hygiene, source size, Skills, frontend build and tests, and Rust linting and tests. Some backend tests start a local HTTP server and need loopback access. No external model account is required for the check suite.

## Contributing

Bug reports, documentation, translations and focused fixes are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Include your OS version, reproduction steps and sanitized diagnostics in bug reports. Discuss larger changes in an issue first.

Start with the [development guide](docs/development.md) and [architecture guide](docs/architecture.md); these detailed guides are currently in Chinese. The repository homepage and contribution/security guidance are available in English.

## License

Mstudio is released under the [MIT License](LICENSE). Third-party libraries and brand assets retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Packaging the application does not replace the redistribution obligations of FFmpeg, GStreamer or their dependencies.
