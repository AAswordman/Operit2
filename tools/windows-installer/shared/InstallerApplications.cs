using System;
using System.ComponentModel;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace Operit.Installer
{
    public static class InstallerApplications
    {
        /// <summary>Uses Windows Restart Manager to close processes holding this installation's executable before file validation.</summary>
        public static void Close(string installFolder, Action<string> log)
        {
            string executable = Path.Combine(UserDataCleanup.Normalize(installFolder), "operit2.exe");
            if (!File.Exists(executable)) return;
            var key = new StringBuilder(33);
            Check(RmStartSession(out uint session, 0, key));
            try
            {
                Check(RmRegisterResources(session, 1, new[] { executable }, 0, IntPtr.Zero, 0, null));
                log("正在关闭占用此安装副本的进程：" + executable);
                // RmForceShutdown lets Windows close applications that do not respond to session shutdown.
                Check(RmShutdown(session, 1, IntPtr.Zero));
                log("Windows Restart Manager 已完成关闭应用。");
            }
            finally { RmEndSession(session); }
        }

        /// <summary>Reports system shutdown failures before the installer changes application files.</summary>
        private static void Check(uint result)
        {
            if (result != 0)
                throw new Win32Exception((int)result, "无法自动关闭正在运行的 Operit2（系统错误 " + result
                    + "）。请右键任务栏右下角的 Operit2 图标，选择“退出”，再重新安装。若仍有后台进程，请在任务管理器中结束 Operit2。");
        }

        /// <summary>Creates an isolated Restart Manager session.</summary>
        [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
        private static extern uint RmStartSession(out uint session, uint flags, StringBuilder key);

        /// <summary>Registers the exact executable resource rather than matching process names.</summary>
        [DllImport("rstrtmgr.dll", CharSet = CharSet.Unicode)]
        private static extern uint RmRegisterResources(uint session, uint fileCount, string[] files,
            uint applicationCount, IntPtr applications, uint serviceCount, string[] services);

        /// <summary>Requests Windows session shutdown for applications using the registered resource.</summary>
        [DllImport("rstrtmgr.dll")]
        private static extern uint RmShutdown(uint session, uint flags, IntPtr callback);

        /// <summary>Releases the Restart Manager session without restarting applications during installation.</summary>
        [DllImport("rstrtmgr.dll")]
        private static extern uint RmEndSession(uint session);
    }
}
