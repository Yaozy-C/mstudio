import type { ProductionTask } from "./types";
export function runProgress(task: ProductionTask): string {
  if (task.status === "CANCEL_REQUESTED") return "已请求取消，正在核实最终状态";
  if (task.error && task.status === "RECEIVING")
    return "生成已完成，结果尚未导入项目";
  if (task.error && ["IN_QUEUE", "IN_PROGRESS"].includes(task.status ?? ""))
    return "进度暂时无法更新，原任务状态待核实";
  if (task.status === "UPLOADING") return "正在准备参考素材";
  if (task.status === "SUBMITTING") return "正在向生成服务提交请求";
  if (task.status === "IN_QUEUE") return "服务已接收请求，正在排队";
  if (task.status === "IN_PROGRESS")
    return task.progress?.message || "生成服务正在处理，等待返回结果";
  if (task.status === "RECEIVING")
    return "生成已完成，正在导入图片或视频到项目";
  if (task.status === "UNKNOWN") return "正在核实提交结果，请勿重复提交";
  return "等待执行";
}
