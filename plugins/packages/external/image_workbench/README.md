# 生图工作台

官方额外插件 `com.operit.image_workbench` 将原先六个独立绘图包收为内部适配器。
在“更多插件”中添加并启用后，侧边栏和工具箱均可打开原生 Compose 工作台，
界面与 AI 调用同一个公共工具。插件随应用附带，默认不启用。

## 统一工具

- `image_workbench:list_methods`：列出生图方式、默认模型、支持的选项与密钥配置状态，不返回密钥。
- `image_workbench:generate_image`：通过 `provider` 选择 `openai`、`qwen`、`zhipu`、`xai`、`minimax`、`nanobanana`。
- `image_workbench:generate_video`：保留原 xAI 视频生成与编辑能力，当前 `provider` 固定为 `xai`。

所有图片请求共用 `prompt`、`model`、`file_name`。数量使用 `count`，参考图片使用
`reference_images`（公网 URL 或 VFS 路径数组），提示词优化使用 `optimize_prompt`。
尺寸统一为 `宽x高`，MiniMax 内部转换为宽高参数；Nano Banana 的 `resolution` 为
`1K`、`2K` 或 `4K`。不支持的参数会报错，避免用户以为某个选项已经生效。
`list_methods` 返回各方式实际支持的字段。

结果统一为 `{success, message, data: {provider, kind, prompt, model, assets, markdown, created_at}}`。
每个 asset 有 `uri` 和 `markdown`，文件通过现有 Host 文件工具写入插件 VFS 目录。
工作台预览、路径和复制引用均使用这些结果。页面会话保留最近 20 次成功结果；文件独立保存。

## 配置

在工作台中选择方式后展开“配置 API Key 与接口”。配置保存在统一的 `image_workbench`
环境变量命名空间，原独立包命名空间中的配置需要在这里重新保存。各方式分别配置，
使用一种方式无需填写其他方式的密钥。支持自定义地址的方式可配置对应接口地址。

MiniMax 和 Nano Banana 的本地参考图会上传到 Beeimg，因此需要配置 `BEEIMG_API_KEY`；
工作台会在参考图输入旁显示此行为。使用公网图片 URL 不需要上传本地文件。
xAI 图生视频通过服务原有 data URL 输入能力处理本地图片。

## 检查与打包

```sh
corepack pnpm install --frozen-lockfile
corepack pnpm run check
corepack pnpm test
corepack pnpm run pack:toolpkg
```

标准插件同步器会打包本目录，并从 external 附带资源中移除已经删除的六个独立入口。
服务端真实生成需要有效密钥，自动测试使用可检查请求与文件写入的 HTTP/Host 夹具。
