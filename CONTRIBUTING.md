# 贡献指南

感谢你为 Operit2 提交改进。Operit2 是一个面向终端的 AI 工作台，仓库同时包含 Rust 核心、各平台 host、CLI、Flutter 应用和构建工具。提交改动前，请先确认改动属于本仓库的维护范围，并保留与现有架构一致的实现方式。

## 开始之前

请先阅读：

- [README.md](README.md)：项目定位、CLI 用法和常用检查命令。
- [BUILDING.md](BUILDING.md)：完整的构建环境和平台工具链要求。
- [LICENSE](LICENSE)：项目采用 GNU Affero General Public License v3.0。

提交安全问题时，请不要公开创建 Issue 或 PR；请通过仓库维护者公布的私下渠道联系维护者。

## 改动范围

- 保持 host 兼容层和 Rust runtime 的边界，不在 Rust 或 Dart 中添加仅为某个平台服务的分支。
- 复用已有的 host API、运行时抽象和平台适配层，避免在 Flutter 侧重复实现文件系统或 HTTP 能力。
- 修改行为时同步更新相邻文档、测试和用户可见的错误信息。
- 第三方代码、生成文件和资源必须保留其原始许可证与版权声明，并在 PR 中注明来源。
- 不提交 API 密钥、访问令牌、签名文件、个人数据或本地构建产物。

## 本地检查

根据改动范围运行相关检查，并在 PR 描述中记录实际运行过的命令及结果。

Rust CLI：

```powershell
cargo check --manifest-path apps/cli/Cargo.toml
cargo run --manifest-path apps/cli/Cargo.toml --bin operit2 -- cli version
```

Flutter 应用：

```powershell
Set-Location apps/flutter/app
fvm flutter pub get --enforce-lockfile
fvm flutter analyze
fvm flutter test
```

Rust 或 Flutter bridge 改动也应按照 [BUILDING.md](BUILDING.md) 中对应平台章节执行检查。涉及构建脚本、发布工具或 Web runtime 的改动，请运行对应目录已有的 smoke test 或验证脚本。

## 分支和提交

1. 从最新的默认分支创建主题分支，每个分支聚焦一个问题。
2. 提交信息使用简短、祈使语气的英文主题，推荐采用 `type(scope): summary` 格式，例如 `fix(cli): preserve chat attachments`。
3. 将格式化、重命名和功能改动分开提交，便于审查。
4. 每个提交都必须包含 DCO 签署行。可以使用以下命令创建签署提交：

   ```powershell
   git commit -s -m "fix(cli): preserve chat attachments"
   ```

   该命令会在提交正文加入 `Signed-off-by: Your Name <your.email@example.com>`。姓名和邮箱应与提交者身份一致。

## 创建 Pull Request

PR 应当：

- 使用清晰的标题，说明改动结果而不是实现过程。
- 描述问题、解决方案、影响范围和验证结果。
- 列出涉及的平台；跨平台能力应说明 host API 和 Flutter/Rust 边界是否变化。
- 为用户可见的行为变化附上截图、日志或复现步骤。
- 说明迁移、配置、数据库或兼容性影响。
- 确认所有提交都包含 DCO 签署行，且 PR 模板中的检查项已经完成。

维护者会关注正确性、跨平台一致性、数据安全、许可证合规性和测试覆盖。审查意见处理完毕后，维护者会合并或要求继续修改；提交 PR 不代表改动一定会被接受。

## 贡献许可

本项目采用 inbound = outbound 的贡献许可原则：你保留自己原创改动的版权，并按照项目当前的 [AGPL-3.0](LICENSE) 许可证向项目及其接收者授权这些改动。项目不要求贡献者转让版权。

提交贡献时，你通过 DCO 签署确认：

- 你有权提交该内容，或已获得必要授权；
- 该内容可以按照项目许可证发布；
- 你理解提交记录会成为项目公开历史的一部分。

DCO 的完整文本见 [Developer Certificate of Origin 1.1](https://developercertificate.org/)。第三方代码不能通过 DCO 规避其原始许可证要求。

## 商业化说明

本项目目前由个人维护。项目维护者可能围绕 Operit2 提供收费的托管、部署、技术支持、定制开发、咨询、培训、企业集成及其他商业服务。

商业化安排不改变已经发布代码和已接受贡献的许可证义务：受 AGPL-3.0 覆盖的代码继续按照 AGPL-3.0 发布，并按该许可证的适用条件履行源代码提供义务；不属于 AGPL 覆盖范围且具有独立版权基础的服务、组件或资源可以采用单独的许可证或商业条款，并会在对应页面明确标注。收费服务不会自动授予客户超出适用许可证范围的源代码权利。

项目未来可能成立公司或由其他运营主体承接商业服务。运营主体变化不会自动改变既有版权归属或许可证；维护者本人拥有的版权资产由维护者通过书面许可或转让交由新主体使用。第三方贡献者的版权不会因运营主体变化而转移，涉及额外授权时需要单独取得相应权利人的同意。

## 行为规范

讨论和审查应当聚焦技术事实，尊重不同背景的参与者。请避免人身攻击、骚扰、歧视性言论、恶意披露个人信息以及故意提交破坏性内容。维护者可以关闭不符合本指南或行为规范的讨论与贡献。
