# mstudio

**A Vibe Video Studio · From mind to motion.**

[简体中文](README.md) | **English**

mstudio is a local video creation studio. Start with an idea, a script or a reference, then write, watch and refine with AI. Keep your script, storyboard, generated media, editing and export together in one project.

A film is rarely something you can describe perfectly in one prompt. A storyboard reveals a composition that needs changing; playback reveals an action that is too slow or music that enters too early. We call this process of talking, watching and adjusting **Vibe Video**. The canvas lays out your ideas and media, the timeline lets you refine individual clips, and creative assistants help with writing and actions. You can take over whenever you need to.

> mstudio is in development, primarily developed and verified on macOS. Windows builds and real-device validation are not complete; Linux does not yet support native desktop preview. AI-generated images, movement and pacing still need your judgment.

## What you can do

1. **Create with your own references**
   Import images, video, audio, text documents and PDFs. Organize them on the canvas or reference specific items in chat so AI can work with your materials. Project media belongs to the current film; the shared library makes assets reusable across projects. Whether a model can understand a material depends on its supported inputs.

2. **Write the story and its sound**
   Edit visuals and action, dialogue and narration, on-screen text, sound and rhythm, and estimated duration in the script workspace. Ask the writer for a draft or write it yourself. Changing the total duration redistributes time across paragraphs using the existing rhythm; you can also adjust paragraphs individually.

3. **Plan shots, then see the storyboard**
   The shot director plans framing, camera position, subject action, camera movement and cut points from the full script. The storyboard artist turns those plans into visible frames. Multiple key frames can describe a complex action within one shot. Paragraphs and shots stay linked so you can return to a specific part and refine it.

4. **Generate and refine media around each shot**
   Shots retain prompts, references and results. Use content references, motion references or first and last frames when the selected model supports them. Select an image and tell AI what to change. Track generation progress, preview results, reference them again and place them on the canvas. Choose the versions you want for the timeline. When a task has trouble, check its status or retry collecting an existing result.

5. **Edit until the rhythm feels right**
   Arrange visuals and sound on a multitrack timeline. Move, split, trim, retime, copy or delete clips; adjust size, position, opacity and volume. Detach audio from video, add audio fades and inspect cut points frame by frame. Reference a specific clip to ask an Agent for help editing it.

6. **Add captions, voiceover, transitions and color**
   Add captions manually, import or export SRT, and adjust font, color and position. Generate voiceover segments using voices installed on your computer, with captions matched to actual speech duration. Add dissolves, slides, wipes and other transitions between adjacent base-layer visuals. Adjust brightness, contrast, saturation and temperature yourself, or ask the transition designer or colorist for help.

7. **Configure your creative team and models**
   A producer, writer, shot director, storyboard artist, media producer, editing and sound specialist, reviewer and other Agents contribute according to their roles. Edit their roles and instructions, assign creative methods through Skills, and configure tool permissions separately. Choose chat and media-generation models independently, using compatible local or online services.

8. **Pick up where you left off and export your film**
   Projects save automatically on your computer. Project memory keeps goals, constraints and confirmed decisions available to the project’s Agents. Export an MP4 locally, preview it and save a copy wherever you need it. Settings supports English and Simplified Chinese, and lets you move your media storage folder with existing files.

## Make your first film

### 1. Open the app and choose your language and models

From the project library, open **General → Language** and select English or 简体中文. The change is immediate and remembered next time. It changes the interface without translating existing scripts, prompts, project names or conversations.

Open **Models → Service connections** to add a service. Add a chat model and set it as the default. Image, video and audio models are managed separately; configure those you need. Supported connections include OpenAI-compatible services, Responses, Gemini, native Claude interfaces and local services using supported protocols.

Remote services require your own account or API key and may charge for usage. Importing media, editing manually and exporting locally do not require AI models.

### 2. Create a project and bring in your goal and references

Select **New project** and name your film. Projects start in portrait at 1080 × 1920; you can change the dimensions and frame rate in project settings.

Open **Media** to import files. Add existing media directly to the timeline or arrange it on the production canvas first. In chat, type `@` to reference media, scripts or shots, and `$` to select a specialist Agent.

For example, tell the producer:

> I want to make a 30-second film for a coffee shop, aimed at people working nearby. It should feel quiet and lived-in. These images show the interior. Help me develop a direction and script first.

### 3. Write the script and review the shot plan

Go to **Script** and ask AI to draft, or write manually. Check the story, dialogue, on-screen text and sound, then select **Plan the whole film’s shots**. Open **Production canvas** to review shots and storyboard frames. Reference anything you want to revise and continue the conversation.

> Give the audience time to see the steam above the cup before cutting to the person. Keep the dialogue after this unchanged.

### 4. Make the visuals and choose usable results

Choose an image or video model for a shot, write a prompt and add references. Set aspect ratio, duration or first and last frames where supported. Review the setup and start generation. Preview the result before deciding whether to refine it, generate another version or add it to the timeline.

If submission status is unclear, check the original task or the provider’s history before generating again to avoid duplicate requests and charges.

### 5. Edit, listen and export

Go to **Film** to adjust order, cut points and sound on the timeline. Add captions, voiceover, transitions and color adjustments as needed. Watch the full film to check picture and sound, then select **Export film → Start export**. When it finishes, preview it and choose **Save as MP4**.

## Everyday controls

| What you want to do | How |
| --- | --- |
| Ask AI to change a specific item | Type `@` in chat, or choose “Reference in chat” on media, shots or clips |
| Choose a specialist | Type `$` in chat to select an Agent |
| Inspect media or a generated result | Double-click canvas content or use its preview action |
| Use media in your film | Drag it onto the timeline or choose “Add to timeline” |
| Split the selected clip | `⌘B`; `Ctrl+B` on Windows |
| Copy / cut / paste clips | `⌘C` / `⌘X` / `⌘V`; use `Ctrl` on Windows |
| Trim before / after the playhead | `⌥[` / `⌥]`; use `Alt` on Windows |
| Undo / redo | `⌘Z` / `⇧⌘Z`; use `Ctrl` on Windows |
| Play / pause | `Space`; see the timeline’s shortcuts for more |

Trimming preserves clip speed. Cutting and deleting leave gaps without automatically moving other clips. Text fields keep their normal text-editing shortcuts.

## Saving, media and privacy

- **Autosave:** Changes save locally after about 1.5 seconds of inactivity. Check the toolbar’s save status. Autosave is not a versioned backup.
- **Project memory:** Maintain goals and key decisions in the current project’s settings. Add notes manually or allow Agents to organize them. Switching models or clearing chat does not clear project memory.
- **Media storage:** Open **General → Storage** to see the location or migrate files to a new folder. Missing files leave shots, timeline positions and references intact. Restore files to their original locations to recover automatically.
- **Backups:** On macOS, the default application data folder is `~/Library/Application Support/local.mstudio.canvas/`. Quit the app before backing up the entire directory. If you moved media storage, also back up the media folder shown in Settings.
- **Remote models:** A local studio does not mean all AI runs locally. Online model requests send prompts and selected references to the configured provider.
- **Credentials:** Credentials currently live in the local database without system-keychain encryption. Do not publish the database, complete data folder or logs containing keys.

## Current limits

AI can still generate incorrect objects, unnatural movement and inconsistent shots. Human selection and refinement remain necessary. Importing references does not mean the app automatically searches for or screens reference videos.

Transitions currently support adjacent, full-frame, opaque base-layer visuals. Duration is 0.05–3 seconds and cannot exceed either adjacent clip. When source handles are insufficient, edge frames are extended. Automatic optical flow, masked occlusion and audio crossfades are not included. Color controls provide basic adjustments, not a complete professional color-management system.

The browser development page is an interface preview. The complete workflow—including file import, native playback, generation tasks and export—requires the desktop app. Browser transition previews currently show hard cuts.

## Get involved

You can currently build and run the desktop app from source. See the [development guide](docs/development.md) for dependencies, startup and packaging, and the [architecture notes](docs/architecture.md) for implementation details.

Contributions are welcome: read the [contributing guide](CONTRIBUTING.md). For bug reports, include your OS version, reproduction steps and redacted error details. See the [security policy](SECURITY.md) for private security reports.

mstudio is licensed under the [MIT License](LICENSE). See [third-party notices](THIRD_PARTY_NOTICES.md) for dependency and brand-icon information.
