using System;
using System.IO;
using System.Linq;
using System.Threading;
using System.Windows;
using System.Web.Script.Serialization;
using Microsoft.Win32;
using WixToolset.Dtf.WindowsInstaller;

namespace Operit.Installer
{
    public sealed class EmbeddedUI : IEmbeddedUI
    {
        private readonly ManualResetEvent start = new ManualResetEvent(false);
        private readonly ManualResetEvent exit = new ManualResetEvent(false);
        private Thread thread;
        private SetupWindow window;
        private Exception startupError;
        private bool committed;
        private bool finalizing;
        private SetupDiagnostics diagnostics;
        private long total;
        private long completed;
        private int step;
        private bool actionData;
        private bool backwards;
        private int phase;
        private double displayedProgress;

        /// <summary>Runs the custom WPF interface for full UI and leaves unattended MSI modes unattended.</summary>
        public bool Initialize(Session session, string resourcePath, ref InstallUIOptions internalUILevel)
        {
            if (((int)internalUILevel & 7) != (int)InstallUIOptions.Full) return false;
            string logDirectory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Operit", "InstallerLogs");
            try
            {
                diagnostics = new SetupDiagnostics(logDirectory, session["OPERIT_RELEASE_VERSION"],
                    session["OriginalDatabase"], session["MsiLogFileLocation"]);
                return InitializeInteractive(session, ref internalUILevel);
            }
            catch (InstallCanceledException) { throw; }
            catch (Exception error)
            {
                string details;
                if (diagnostics != null)
                {
                    diagnostics.AddFailure(0, error.ToString());
                    details = error.Message + "\r\n\r\n诊断日志：" + diagnostics.LogPath
                        + "\r\n请把此错误和日志提供给开发者。";
                }
                else details = "无法创建安装诊断日志，安装尚未开始。\r\n日志目录：" + logDirectory
                    + "\r\n原始错误：" + error.Message + "\r\n请检查该目录的写入权限和磁盘空间。\r\n"
                    + "也可以在命令提示符中运行下列命令，记录 Windows Installer 的具体错误：\r\n"
                    + "msiexec.exe /i \"" + session["OriginalDatabase"] + "\" /L*V \"%USERPROFILE%\\Desktop\\Operit2-install.log\"";
                MessageBox.Show("安装界面无法启动。\r\n\r\n" + details
                    + "\r\n\r\n在此提示框按 Ctrl+C 可复制完整内容。", "Operit2 安装失败", MessageBoxButton.OK, MessageBoxImage.Error);
                throw new InstallerException("Operit2 安装初始化失败：" + error.Message, error);
            }
        }

        /// <summary>Initializes the interactive session after diagnostic logging has been established.</summary>
        private bool InitializeInteractive(Session session, ref InstallUIOptions internalUILevel)
        {
            string folder = ResolveInstallationFolder(session["INSTALLFOLDER"]);
            string version = session["OPERIT_RELEASE_VERSION"];
            bool installed = session.EvaluateCondition("Installed");
            bool removing = string.Equals(session["REMOVE"], "ALL", StringComparison.OrdinalIgnoreCase);
            var operation = SetupOperationSelection.Resolve(removing,
                Version.Parse(session["ProductVersion"]),
                ProductInstallation.GetRelatedProducts(session["UpgradeCode"])
                    .Where(product => product.IsInstalled).Select(product => product.ProductVersion));
            thread = new Thread(() => RunWindow(version, folder, operation, installed));
            thread.SetApartmentState(ApartmentState.STA);
            thread.Start();
            if (WaitHandle.WaitAny(new WaitHandle[] { start, exit }) == 1)
            {
                thread.Join();
                start.Dispose();
                exit.Dispose();
                if (startupError != null) throw new InstallerException("Operit2 安装界面无法启动。", startupError);
                throw new InstallCanceledException();
            }
            session["INSTALLFOLDER"] = window.InstallFolder;
            diagnostics.Write("Selection", "操作：" + window.Operation + "；安装目录：" + window.InstallFolder);
            if (installed && window.Operation == SetupOperation.Repair)
            {
                session["REINSTALL"] = "ALL";
                session["REINSTALLMODE"] = "amus";
            }
            if (window.Operation == SetupOperation.Remove) session["REMOVE"] = "ALL";
            session["OPERIT_DELETE_USER_DATA"] = window.DeleteData ? "1" : "";
            session["OPERIT_CLEANUP_INTERACTIVE"] = window.DeleteData ? "1" : "";
            if (window.DeleteData) session["OPERIT_CLEANUP_PLAN"] = new JavaScriptSerializer().Serialize(window.DataCleanupPlan);
            internalUILevel = InstallUIOptions.Silent;
            return true;
        }

        /// <summary>Resolves an explicit destination, an existing installation, or the new-install default.</summary>
        private static string ResolveInstallationFolder(string requestedFolder)
        {
            if (!string.IsNullOrEmpty(requestedFolder)) return requestedFolder;
            using (var key = Registry.CurrentUser.OpenSubKey(@"Software\Operit\Operit2\Installer"))
            {
                if (key == null)
                    return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Programs", "Operit2");
                string installedFolder = key.GetValue("InstallLocation") as string;
                if (string.IsNullOrEmpty(installedFolder))
                    throw new InstallerException("现有 Operit2 安装信息不完整：缺少安装目录。");
                return installedFolder;
            }
        }

        /// <summary>Creates the UI on its own STA thread and always releases initialization waiters.</summary>
        private void RunWindow(string version, string folder, SetupOperation operation, bool installed)
        {
            try
            {
                window = new SetupWindow(version, folder, operation, installed, start, diagnostics);
                new Application { ShutdownMode = ShutdownMode.OnMainWindowClose }.Run(window);
            }
            catch (Exception error) { startupError = error; }
            finally { exit.Set(); }
        }

        /// <summary>Processes transaction messages without exposing the legacy Windows Installer dialogs.</summary>
        public MessageResult ProcessMessage(InstallMessage messageType, Record record,
            MessageButtons buttons, MessageIcon icon, MessageDefaultButton defaultButton)
        {
            if (record != null && (messageType == InstallMessage.Info || messageType == InstallMessage.Warning))
                diagnostics.Write(messageType.ToString(), record.ToString());
            if (messageType == InstallMessage.ActionStart && record != null)
            {
                actionData = false;
                string action = record.GetString(1);
                diagnostics.StartAction(action, record.FieldCount >= 2 ? record.GetString(2) : "");
                if (action == "OperitFinishRemoval" || action == "OperitRegisterShellEntries")
                {
                    finalizing = true;
                    window.Dispatcher.Invoke(new Action(window.BeginFinalization));
                }
                if (action == "OperitInstallCompleted") committed = true;
                string label = null;
                if (action == "OperitCloseApplications") label = "正在关闭已运行的 Operit2…";
                if (action == "OperitFinishRemoval") label = "正在清理应用入口及所选用户数据…";
                if (action == "InstallFiles") label = "正在安装应用文件…";
                if (action == "CreateShortcuts") label = "正在创建快捷方式…";
                if (action == "RemoveFiles") label = "正在移除应用文件…";
                if (action == "RegisterProduct") label = "正在完成安装…";
                UpdateProgress(label);
            }
            if (messageType == InstallMessage.Progress && record != null && record.FieldCount >= 2)
            {
                int type = record.GetInteger(1);
                if (type == 0 && record.FieldCount >= 4)
                {
                    phase++;
                    total = record.GetInteger(2);
                    backwards = record.GetInteger(3) != 0;
                    completed = backwards ? total : 0;
                    actionData = false;
                }
                if (type == 1 && record.FieldCount >= 3)
                {
                    step = record.GetInteger(2);
                    actionData = record.GetInteger(3) != 0;
                }
                if (type == 2) completed += (backwards ? -1L : 1L) * record.GetInteger(2);
                if (type == 3) total += record.GetInteger(2);
                UpdateProgress(null);
            }
            if (messageType == InstallMessage.ActionData && actionData)
            {
                completed += (backwards ? -1L : 1L) * step;
                UpdateProgress(null);
            }
            if (messageType == InstallMessage.Error || messageType == InstallMessage.FatalExit)
            {
                committed = false;
                int code = record != null && record.FieldCount >= 1 ? record.GetInteger(1) : 0;
                diagnostics.AddFailure(code, record == null ? "Windows Installer 报告致命错误，但未提供错误记录。" : record.ToString());
                window.Dispatcher.Invoke(new Action(window.ShowFailure));
            }
            if (messageType == InstallMessage.FilesInUse || messageType == InstallMessage.RMFilesInUse)
            {
                committed = false;
                diagnostics.AddFailure(0, "Operit2 或相关文件正在使用中。请退出 Operit2 的所有窗口、托盘和后台进程后重新运行安装程序。\r\n"
                    + (record == null ? "Windows Installer 未提供占用进程详情。" : record.ToString()), filesInUse: true);
                window.Dispatcher.Invoke(new Action(window.ShowFailure));
                return MessageResult.Cancel;
            }
            bool canceled = (bool)window.Dispatcher.Invoke(new Func<bool>(() => window.CancelRequested));
            return canceled && !committed && !finalizing ? MessageResult.Cancel : MessageResult.OK;
        }

        /// <summary>Converts MSI planning and execution ticks into a stable visible progress value.</summary>
        private void UpdateProgress(string action)
        {
            double value = total > 0 ? Math.Max(0, Math.Min(1, (double)completed / total)) : 0;
            value = phase <= 1 ? value * 10 : 10 + value * 89;
            displayedProgress = Math.Max(displayedProgress, value);
            window.Dispatcher.Invoke(new Action(() => window.ReportProgress(displayedProgress, action)));
        }

        /// <summary>Displays the final transaction result and waits until the user closes the window.</summary>
        public void Shutdown()
        {
            bool success = committed && !diagnostics.HasErrors;
            diagnostics.Write("Result", success ? "操作完成" : "操作未完成；用户请求取消：" + window.CancelRequested);
            window.Dispatcher.BeginInvoke(new Action(() => window.Complete(success)));
            thread.Join();
            start.Dispose();
            exit.Dispose();
        }
    }
}
