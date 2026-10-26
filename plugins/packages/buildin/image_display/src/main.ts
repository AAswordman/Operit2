/** Teaches the model the existing Markdown image contract and shared storage location. */
export function onSystemPromptCompose(
  event: ToolPkg.SystemPromptComposeHookEvent
): ToolPkg.PromptHookObjectResult | null {
  if (event.eventName !== "after_compose_system_prompt") return null;
  const { systemPrompt, useEnglish } = event.eventPayload;
  if (typeof systemPrompt !== "string") throw new Error("Image display requires a system prompt");
  const directory = ToolPkg.getSpaceDataDir();
  const guidance = useEnglish === true
    ? [
        "IMAGE DISPLAY:",
        `- Save images for display in this shared VFS directory: ${directory}/`,
        "- Use the available file tools to copy, download, or write the actual image there. Confirm success before referencing the file; never invent an image path.",
        "- Use a unique filename containing only ASCII letters, digits, hyphens, underscores, and its image extension. Keep existing images so saved conversations retain their references.",
        `- Display it using standard Markdown: ![Image description](${directory}/image-unique.png)`,
        "- Replace the example filename with the actual saved filename. Output the Markdown directly on its own line, outside code blocks. Use the absolute VFS path returned by the file tools.",
      ].join("\n")
    : [
        "图片展示：",
        `- 需要展示的图片应保存到共享 VFS 目录：${directory}/`,
        "- 使用已有文件工具将真实图片复制、下载或写入该目录，确认成功后再引用，禁止编造图片路径。",
        "- 使用唯一文件名，文件名仅包含英文字母、数字、连字符、下划线和图片扩展名。保留已有图片，确保历史消息中的引用持续有效。",
        `- 使用标准 Markdown 展示：![图片说明](${directory}/image-unique.png)`,
        "- 将示例文件名替换为实际保存的文件名。图片语法单独一行直接输出，放在代码块外，使用文件工具返回的 VFS 绝对路径。",
      ].join("\n");
  return { systemPrompt: `${systemPrompt}\n\n${guidance}` };
}

/** Registers image instructions through the existing system prompt hook. */
export function registerToolPkg(): boolean {
  ToolPkg.registerSystemPromptComposeHook({
    id: "image_display_prompt",
    function: onSystemPromptCompose,
  });
  return true;
}
