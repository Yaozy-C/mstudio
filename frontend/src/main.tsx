import ReactDOM from "react-dom/client";
import { Theme } from "@radix-ui/themes";
import "@radix-ui/themes/styles.css";
import "./styles/base.css";
import "./styles/workspace.css";
import "./styles/timeline.css";
import "./styles/dialogs.css";
import "./styles/studio.css";
import "./styles/preview.css";
import "./styles/workspace-overlays.css";
import "./styles/agent.css";
import "./styles/editing.css";
import "./styles/attachments.css";
import "./styles/node-content.css";
import "./styles/inline-text.css";
import "./styles/interaction.css";
import { App } from "./workspace/App";
import { SafeExit } from "./workspace/SafeExit";
import "./styles/production-desk.css";
import "./styles/production-surfaces.css";
import "./styles/frame-theme.css";
import "./styles/frame-workspace.css";
import "./styles/frame-timeline.css";
import "./styles/timeline-states.css";
import "./styles/frame-media.css";
import "./styles/object-menu.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <Theme appearance="light" accentColor="tomato" radius="small">
    <App />
    <SafeExit />
  </Theme>,
);
