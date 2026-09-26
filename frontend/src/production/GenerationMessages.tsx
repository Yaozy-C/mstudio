import { useContext } from "react";
import { MessageContext } from "../assistant/AgentMessage";
import { TaskEntry } from "./TaskEntry";
export function GenerationMessages({ turnId }: { turnId: string }) {
  const context = useContext(MessageContext)!;
  const tasks = context.canvas?.runs.filter((t) => t.turnId === turnId) ?? [];
  if (!tasks.length) return null;
  return (
    <div className="conversation-generation-results">
      {context.canvas &&
        tasks.map((task) => (
          <TaskEntry key={task.key} task={task} canvas={context.canvas!} />
        ))}
    </div>
  );
}
