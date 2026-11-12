using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;

namespace Operit.Installer
{
    public sealed class SetupProblem
    {
        public string Title { get; set; }
        public string Reason { get; set; }
        public string Steps { get; set; }
    }

    public sealed class SetupDiagnostics
    {
        private readonly List<string> failures = new List<string>();
        private readonly List<int> codes = new List<int>();
        private readonly object sync = new object();
        private int firstCode;
        private string firstAction;
        private bool firstFilesInUse;
        public string LogPath { get; }
        public string MsiLogPath { get; }
        public string PackagePath { get; }
        public string Version { get; }
        public string CurrentAction { get; private set; } = "初始化安装界面";
        public string LoggingError { get; private set; }
        public bool HasErrors { get { lock (sync) return failures.Count != 0; } }

        /// <summary>Creates a unique diagnostic file outside application and user-data removal directories.</summary>
        public SetupDiagnostics(string directory, string version, string packagePath, string msiLogPath)
        {
            Version = version;
            PackagePath = packagePath;
            MsiLogPath = msiLogPath;
            Directory.CreateDirectory(directory);
            LogPath = Path.Combine(directory, "Operit2-setup-" + DateTime.Now.ToString("yyyyMMdd-HHmmss")
                + "-" + Guid.NewGuid().ToString("N") + ".log");
            File.WriteAllText(LogPath, "Operit2 安装诊断\r\n版本：" + version + "\r\n安装包：" + packagePath
                + "\r\nWindows：" + Environment.OSVersion + "\r\n进程位数：" + (Environment.Is64BitProcess ? "64" : "32")
                + "\r\nTEMP：" + Path.GetTempPath() + "\r\nMSI 详细日志：" + msiLogPath + "\r\n", new UTF8Encoding(true));
        }

        /// <summary>Records the exact MSI action before its messages arrive.</summary>
        public void StartAction(string action, string description)
        {
            lock (sync)
            {
                CurrentAction = action;
                Write("ActionStart", action + " — " + description);
            }
        }

        /// <summary>Preserves every failure so later transaction errors cannot overwrite the original cause.</summary>
        public void AddFailure(int code, string message, bool filesInUse = false)
        {
            lock (sync)
            {
                if (failures.Count == 0)
                {
                    firstCode = code;
                    firstAction = CurrentAction;
                    firstFilesInUse = filesInUse;
                }
                if (code > 0 && !codes.Contains(code)) codes.Add(code);
                string failure = (code > 0 ? "错误码：" + code + "\r\n" : "")
                    + "发生步骤：" + CurrentAction + "\r\n原始错误：" + message;
                failures.Add(failure);
                Write("Error", failure);
            }
        }

        /// <summary>Shows only a concrete explanation and the next useful actions on the default error screen.</summary>
        public SetupProblem Problem()
        {
            lock (sync)
            {
                string title = "安装失败";
                string reason = "安装程序未能完成操作。";
                string steps = "请点击“复制错误信息”，将信息和本次日志发给开发者排查。";
                if (firstAction == "OperitCloseApplications" || firstFilesInUse)
                {
                    title = "Operit2 还在运行";
                    reason = "未能自动退出，占用的文件无法替换。";
                    steps = "1. 右键任务栏右下角的 Operit2 图标，选择“退出”。\n2. 仍无法退出时，在任务管理器中结束 Operit2，然后重新安装。";
                }
                else switch (firstCode)
                {
                    case 2502:
                    case 2503:
                        title = "Windows 安装服务出错";
                        reason = firstCode == 2502 ? "Windows 安装服务在收尾时失败。" : "Windows 安装服务未能执行安装步骤。";
                        steps = "1. 重启电脑后，重新打开安装包。\n2. 仍然报错时，点击“复制错误信息”，将信息和本次日志发给开发者。";
                        break;
                    case 5: case 1303: case 1310: case 1402: case 1406: case 1925:
                        title = "没有权限写入安装文件";
                        reason = "Windows 拒绝访问安装所需的文件夹、文件或注册表。";
                        steps = "1. 首次安装时使用默认安装位置。\n2. 仍然失败时，将日志交给电脑管理员，修复日志中被拒绝访问的具体位置。";
                        break;
                    case 112: case 1307:
                        title = "磁盘空间不足";
                        reason = "安装目录或系统临时目录所在磁盘没有足够空间。";
                        steps = "清理系统盘和安装盘的空间，然后重新安装。";
                        break;
                    case 1308: case 1311: case 1335: case 1619: case 1620:
                        title = "安装包无法读取";
                        reason = "安装文件可能没有下载完整，或已被损坏、隔离。";
                        steps = "从官方发布页重新下载 MSI，保存到电脑本机文件夹后打开。";
                        break;
                    case 1500: case 1618:
                        title = "另一个安装任务正在运行";
                        reason = "Windows 正在安装其他软件或更新系统。";
                        steps = "等待其他安装和 Windows 更新完成后重试；仍然提示时重启电脑。";
                        break;
                    case 1601: case 1719:
                        title = "Windows 安装服务不可用";
                        reason = "系统无法启动或访问 Windows Installer 服务。";
                        steps = "先重启电脑；仍然失败时，请电脑管理员检查 Windows Installer 服务是否被禁用。";
                        break;
                    case 1625:
                        title = "电脑的安全策略禁止安装";
                        reason = "本次安装被 Windows 或单位管理策略阻止。";
                        steps = "请将错误信息和日志交给电脑管理员处理。";
                        break;
                    case 1633:
                        title = "安装包不支持此系统";
                        reason = "此安装包需要 Windows 10 1809 或更新版本的 64 位 Windows。";
                        steps = "请检查 Windows 版本，并使用匹配系统的安装包。";
                        break;
                    case 1612: case 1706:
                        title = "找不到原来的安装包";
                        reason = "修复、升级或卸载需要原版本的 MSI 文件。";
                        steps = "下载当前已安装版本对应的官方 MSI，在系统要求时选择该文件。";
                        break;
                    case 1638:
                        title = "已安装其他版本";
                        reason = "电脑中已有与此安装包冲突的 Operit2 版本。";
                        steps = "在 Windows 设置 → 应用中查看 Operit2 版本，下载相同或更新版本的安装包。";
                        break;
                }
                if (firstCode > 0) reason += "（错误 " + firstCode + "）";
                return new SetupProblem { Title = title, Reason = reason, Steps = steps };
            }
        }

        /// <summary>Flushes each diagnostic entry and exposes any write failure to the user.</summary>
        public void Write(string kind, string message)
        {
            lock (sync)
            {
                try
                {
                    File.AppendAllText(LogPath, DateTime.Now.ToString("O") + " [" + kind + "] " + message + "\r\n", Encoding.UTF8);
                }
                catch (Exception error) when (error is IOException || error is UnauthorizedAccessException)
                {
                    LoggingError = "诊断日志写入失败：" + LogPath + "\r\n" + error.Message
                        + "\r\n请使用“复制错误信息”保存当前信息，并检查磁盘空间和该目录的写入权限。";
                }
            }
        }

        /// <summary>Builds a selectable support report with instructions, original failures, and real log locations.</summary>
        public string Describe()
        {
            lock (sync) return DescribeReport();
        }

        /// <summary>Formats a consistent snapshot while the diagnostic state is locked.</summary>
        private string DescribeReport()
        {
            var text = new StringBuilder("Operit2 安装诊断\r\n版本：" + Version + "\r\n安装包：" + PackagePath + "\r\n\r\n");
            if (HasErrors)
            {
                text.AppendLine("检测到的错误码：" + (codes.Count == 0 ? "Windows Installer 未提供数字错误码" : string.Join("、", codes)));
                text.AppendLine("\r\n首次错误\r\n" + failures[0]);
                foreach (var group in codes.GroupBy(Guidance))
                    text.AppendLine("\r\n处理建议（" + string.Join(" / ", group) + "）\r\n" + group.Key);
                if (codes.Count == 0) text.AppendLine("请按下面的原始错误检查文件占用、安装目录和系统权限；无法解决时请提供本页详情及日志。");
                text.AppendLine("\r\n原始错误（按发生顺序保留）\r\n" + string.Join("\r\n\r\n", failures));
            }
            else text.AppendLine("尚未收到 Windows Installer 的具体错误。当前步骤：" + CurrentAction);
            text.AppendLine("\r\n诊断日志：" + LogPath);
            text.AppendLine("MSI 详细日志：" + (string.IsNullOrWhiteSpace(MsiLogPath)
                ? "Windows Installer 未提供日志路径。请按下方命令生成详细日志。" : MsiLogPath));
            if (LoggingError != null) text.AppendLine("\r\n" + LoggingError);
            text.AppendLine("\r\n需要反馈时：点击“复制错误信息”，并附上诊断日志和 MSI 详细日志。发送前可检查其中的用户名及本机路径。");
            text.AppendLine("\r\n安装窗口出现前就报错，或需要重新采集完整日志时：\r\n"
                + "1. 按 Win+R，输入 cmd，按回车。\r\n2. 输入以下命令（把引号内的路径改为实际下载的 MSI 文件路径）：\r\n"
                + "msiexec.exe /i \"" + PackagePath + "\" /L*V \"%USERPROFILE%\\Desktop\\Operit2-install.log\"\r\n"
                + "3. 按安装界面操作。结束后在桌面找到 Operit2-install.log；桌面被重定向时，请把日志参数改为你有写入权限的完整路径。");
            return text.ToString();
        }

        /// <summary>Maps documented MSI codes to explanations without inferring causes from message text.</summary>
        public static string Guidance(int code)
        {
            switch (code)
            {
                case 2502:
                case 2503:
                    return "Windows Installer 安装事务状态异常。2502 表示在没有进行中的安装事务时调用了 InstallFinalize；2503 表示未标记为正在安装时调用了 RunScript。\r\n"
                        + "仅凭此代码无法确认根因；请同时查看前面发生的错误和 MSI 详细日志。\r\n"
                        + "1. 保存本页详情和日志，关闭其他安装程序，重启 Windows 后用当前登录账户重试。\r\n"
                        + "2. 将 MSI 下载到本机文件夹后打开，首次安装使用默认安装位置；不要从压缩包、网络共享或无写入权限的目录运行。\r\n"
                        + "3. 按 Win+R，输入 %TEMP%，尝试新建并删除一个文本文件。若提示拒绝访问，让管理员检查该用户临时目录的权限；同时检查磁盘剩余空间。\r\n"
                        + "4. 查看 Windows 安全中心的保护历史记录或单位安全软件的拦截记录。只有日志确认权限问题时，才请管理员检查日志中明确拒绝访问的目录（包括 Windows\\Temp 或 Windows\\Installer）。不要删除 Windows\\Installer，也不要给 Everyone 开放完全控制。\r\n"
                        + "5. Operit2 默认仅为当前用户安装。需要管理员协助时请提供日志，由管理员确认处理方式，避免切换账户后装到另一个用户下。";
                case 5:
                case 1303:
                case 1310:
                case 1402:
                case 1406:
                case 1925:
                    return "无法访问所需文件夹、文件或注册表项。\r\n1. 从原始错误找到被拒绝访问的具体路径或注册表项。\r\n2. 首次安装使用默认的当前用户目录；确认该目录有写入权限。\r\n3. 检查安全软件拦截记录；公司电脑请让管理员按日志修正权限，不要更改整个系统目录的权限。";
                case 112:
                case 1307:
                    return "磁盘空间不足。\r\n1. 检查安装目录和系统盘的剩余空间，MSI 解压也会使用系统临时目录。\r\n2. 使用 Windows 设置中的存储清理释放空间，再重新运行安装程序。";
                case 1308:
                case 1311:
                case 1335:
                case 1619:
                case 1620:
                    return "安装包无法读取、内容缺失或损坏。\r\n1. 删除这份下载不完整的 MSI，从 Operit2 官方发布页重新下载。\r\n2. 保存到本机文件夹后运行，检查安全软件是否隔离了文件。\r\n3. 再次失败时提供安装包文件名、下载来源和日志。";
                case 1500:
                case 1618:
                    return "另一个 Windows 安装任务正在运行。\r\n1. 等待其他软件安装或 Windows 更新结束。\r\n2. 关闭其他安装窗口后重试；仍然提示时重启 Windows。不要强行结束正在更新系统的安装服务。";
                case 1601:
                case 1719:
                    return "无法访问 Windows Installer 服务。\r\n1. 重启 Windows 后重试。\r\n2. 按 Win+R，输入 services.msc，查看 Windows Installer 服务是否被禁用；请管理员检查服务及系统策略。";
                case 1602:
                    return "安装被取消。重新打开 MSI 可再次执行；若没有主动取消，请查看前面的具体错误和日志。";
                case 1603:
                    return "Windows Installer 报告安装失败（汇总错误）。这个代码本身不能说明根因。\r\n1. 先处理本页其他错误码和原始错误。\r\n2. 打开 MSI 详细日志，查找 Return value 3，并保留它前面的错误上下文。\r\n3. 将本页详情与日志提供给开发者，不需要反复尝试相同安装。";
                case 1638:
                    return "已存在冲突版本。\r\n在 Windows 设置 → 应用 → 已安装的应用中确认 Operit2 版本，并下载相同或更新版本的安装包；移除现有版本前先保留并备份用户数据。";
                case 1625:
                    return "安装被系统策略禁止。请联系电脑管理员，并提供此错误和日志；需要管理员检查软件安装限制或单位安全策略。";
                case 1633:
                    return "此安装包不支持当前系统。请确认使用 Windows 10 1809 或更新版本的 64 位 Windows，并下载匹配系统的安装包。";
                case 1612:
                case 1706:
                    return "修复、升级或卸载需要的原始安装源不可用。请下载当前已安装版本对应的官方 MSI，并在系统要求时指定该文件；仍失败请提供日志，不要手工删除 Windows Installer 缓存或产品登记。";
                case 1721:
                case 1722:
                case 1723:
                    return "安装所需的自定义操作执行失败。查看原始错误中的操作名称和日志里该操作的异常；关闭 Operit2 的所有窗口及后台进程后重试，仍失败请将日志发给开发者。";
                default:
                    return "当前错误尚无专门的处理说明。请保留下面的原始错误和出错步骤，打开 MSI 详细日志查看上下文，并将本页详情及日志提供给开发者。";
            }
        }
    }
}
