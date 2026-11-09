using System;
using System.IO;
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
        private string lastError = "安装未完成。请重新运行安装程序并检查 Windows Installer 日志。";
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
            string folder = ResolveInstallationFolder(session["INSTALLFOLDER"]);
            string version = session["OPERIT_RELEASE_VERSION"];
            bool installed = session.EvaluateCondition("Installed");
            bool removing = string.Equals(session["REMOVE"], "ALL", StringComparison.OrdinalIgnoreCase);
            thread = new Thread(() => RunWindow(version, folder, installed, removing));
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
            if (window.Operation == SetupOperation.Repair)
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
        private void RunWindow(string version, string folder, bool installed, bool removing)
        {
            try
            {
                window = new SetupWindow(version, folder, installed, removing, start);
                new Application { ShutdownMode = ShutdownMode.OnMainWindowClose }.Run(window);
            }
            catch (Exception error) { startupError = error; }
            finally { exit.Set(); }
        }

        /// <summary>Processes transaction messages without exposing the legacy Windows Installer dialogs.</summary>
        public MessageResult ProcessMessage(InstallMessage messageType, Record record,
            MessageButtons buttons, MessageIcon icon, MessageDefaultButton defaultButton)
        {
            if (messageType == InstallMessage.ActionStart && record != null)
            {
                actionData = false;
                string action = record.GetString(1);
                if (action == "OperitFinishRemoval" || action == "OperitRegisterShellEntries")
                {
                    finalizing = true;
                    window.Dispatcher.Invoke(new Action(window.BeginFinalization));
                }
                if (action == "OperitInstallCompleted") committed = true;
                string label = null;
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
                lastError = record?.ToString();
            }
            if (messageType == InstallMessage.FilesInUse || messageType == InstallMessage.RMFilesInUse)
            {
                lastError = "Operit2 或相关文件正在使用中。请关闭 Operit2 后重新运行安装程序。";
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
            window.Dispatcher.BeginInvoke(new Action(() => window.Complete(committed, lastError)));
            thread.Join();
            start.Dispose();
            exit.Dispose();
        }
    }
}
