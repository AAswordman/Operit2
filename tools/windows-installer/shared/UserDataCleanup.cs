using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Web.Script.Serialization;
using Microsoft.Win32;

namespace Operit.Installer
{
    public sealed class CleanupPlan
    {
        public string[] Directories { get; set; }
        public string BootstrapPath { get; set; }
        public string BootstrapContent { get; set; }

        /// <summary>Shows the exact deletion scope, including data shared by other copies.</summary>
        public string Describe()
        {
            return "将永久删除以下目录及其全部内容：\n\n" + string.Join("\n", Directories) +
                "\n\n同时删除 Windows 凭据管理器中的 Operit2: 凭据。\n" +
                "包含所有本机身份的聊天、设置、插件、工作区、浏览器数据和运行环境。\n" +
                "安装版与压缩包版共用这些数据，其他副本也会失去这些本机数据。\n" +
                "不会向其他设备或云端发送删除指令。请先导出需要保留的数据。";
        }
    }

    public sealed class UserDataCleanup
    {
        private readonly string roaming;
        private readonly string local;
        private readonly string installFolder;
        private readonly string[] protectedPaths;
        private readonly Action clearCredentials;
        private readonly Action ensureStopped;

        /// <summary>Uses the current user's known folders for both installer and standalone cleanup.</summary>
        public UserDataCleanup(string installFolder) : this(
            Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), installFolder,
            new[] { Environment.SpecialFolder.UserProfile, Environment.SpecialFolder.DesktopDirectory,
                Environment.SpecialFolder.MyPictures, Environment.SpecialFolder.MyMusic, Environment.SpecialFolder.MyVideos,
                Environment.SpecialFolder.MyDocuments, Environment.SpecialFolder.Windows,
                Environment.SpecialFolder.ProgramFiles, Environment.SpecialFolder.ProgramFilesX86,
                Environment.SpecialFolder.CommonApplicationData }.Select(Environment.GetFolderPath)) { }

        /// <summary>Accepts explicit known folders so filesystem safety can be verified in isolation.</summary>
        public UserDataCleanup(string roaming, string local, string installFolder, IEnumerable<string> protectedPaths)
            : this(roaming, local, installFolder, protectedPaths, DeleteCredentials, EnsureOperitStopped) { }

        /// <summary>Injects credential deletion for isolated tests that never touch a real credential store.</summary>
        internal UserDataCleanup(string roaming, string local, string installFolder, IEnumerable<string> protectedPaths,
            Action clearCredentials, Action ensureStopped)
        {
            this.roaming = Normalize(roaming);
            this.local = Normalize(local);
            this.installFolder = Normalize(installFolder);
            this.protectedPaths = protectedPaths.Where(path => !string.IsNullOrEmpty(path))
                .Concat(new[] { this.roaming, this.local, this.installFolder,
                    Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), "Downloads") })
                .Select(Normalize).ToArray();
            this.clearCredentials = clearCredentials;
            this.ensureStopped = ensureStopped;
        }

        /// <summary>Reads the authoritative bootstrap record and validates all roots before any deletion.</summary>
        public CleanupPlan CreatePlan()
        {
            var appRoot = Path.Combine(roaming, "Operit2");
            var bootstrap = Path.Combine(appRoot, "client", "link", "local_runtime_storage.json");
            EnsureNoLinkedAncestor(bootstrap);
            string content = File.Exists(bootstrap) ? File.ReadAllText(bootstrap) : null;
            var roots = new List<string>();
            if (content != null)
            {
                var config = new JavaScriptSerializer().Deserialize<Dictionary<string, object>>(content);
                if (config == null || !config.TryGetValue("confirmed", out var confirmed) || !(confirmed is bool))
                    throw new InvalidDataException("启动配置无效，无法确定自定义数据目录。请先修复配置。 ");
                if ((bool)confirmed)
                {
                    roots.Add(ConfiguredRoot(config, "runtimeRoot"));
                    roots.Add(ConfiguredRoot(config, "workspaceRoot"));
                }
            }
            roots.Add(appRoot);
            roots.Add(Path.Combine(local, "app.operit", "Operit2"));
            roots.Add(Path.Combine(local, "Operit2"));
            roots.Add(Path.Combine(local, "webview_all_windows", "operit2"));
            var normalized = roots.Select(Normalize).Distinct(StringComparer.OrdinalIgnoreCase).ToArray();
            foreach (var root in normalized) ValidateRoot(root);
            // Keep the bootstrap record until every other directory has been removed successfully.
            var ordered = normalized.Where(root => !normalized.Any(other =>
                !Same(root, other) && IsWithin(root, other)))
                .OrderBy(root => IsWithin(bootstrap, root) ? 1 : 0).ThenBy(root => root, StringComparer.OrdinalIgnoreCase).ToArray();
            return new CleanupPlan { Directories = ordered, BootstrapPath = bootstrap, BootstrapContent = content };
        }

        /// <summary>Rejects changed configuration, running Operit processes, links in roots, and locked files.</summary>
        public void ValidatePlan(CleanupPlan plan)
        {
            var current = CreatePlan();
            if (plan == null || !current.Directories.SequenceEqual(plan.Directories, StringComparer.OrdinalIgnoreCase)
                || current.BootstrapContent != plan.BootstrapContent || !Same(current.BootstrapPath, plan.BootstrapPath))
                throw new InvalidOperationException("数据目录配置已改变，请重新打开清理界面并确认。 ");
            ensureStopped();
            foreach (var root in plan.Directories)
            {
                ValidateRoot(root);
                if (Directory.Exists(root)) CheckTree(root);
                else if (File.Exists(root)) throw new IOException("数据目录被文件占用：" + root);
            }
        }

        /// <summary>Deletes only a freshly validated plan and reports all failures to the caller.</summary>
        public void DeleteData(CleanupPlan plan)
        {
            ValidatePlan(plan);
            clearCredentials();
            foreach (var root in plan.Directories)
                if (Directory.Exists(root)) DeleteTree(root, plan.BootstrapPath);
        }

        /// <summary>Requires every Operit runtime to exit before shared data can be modified.</summary>
        public static void EnsureOperitStopped()
        {
            var processes = Process.GetProcessesByName("operit2");
            try
            {
                if (processes.Length != 0)
                    throw new InvalidOperationException("请先退出所有 Operit2 窗口、后台进程及命令行，再继续。 ");
            }
            finally { foreach (var process in processes) process.Dispose(); }
        }

        /// <summary>Reads an explicit absolute root from a confirmed bootstrap record.</summary>
        private static string ConfiguredRoot(Dictionary<string, object> config, string key)
        {
            if (!config.TryGetValue(key, out var value) || !(value is string path) || string.IsNullOrWhiteSpace(path))
                throw new InvalidDataException("启动配置缺少有效目录：" + key);
            return Normalize(path);
        }

        /// <summary>Rejects roots that could erase the system, a user folder, or the running cleanup utility.</summary>
        private void ValidateRoot(string root)
        {
            root = Normalize(root);
            if (Same(root, Path.GetPathRoot(root)) || protectedPaths.Any(path => IsWithin(path, root))
                || IsWithin(root, Environment.GetFolderPath(Environment.SpecialFolder.Windows)))
                throw new InvalidOperationException("拒绝清理受保护的目录：" + root);
            EnsureNoLinkedAncestor(root);
        }

        /// <summary>Accepts only fully qualified ordinary Windows paths and normalizes trailing separators.</summary>
        public static string Normalize(string path)
        {
            if (string.IsNullOrWhiteSpace(path) || !Path.IsPathRooted(path)
                || (path.Length >= 2 && path[1] == ':' && (path.Length < 3 || path[2] != '\\' && path[2] != '/'))
                || path.StartsWith(@"\\?\", StringComparison.Ordinal) || path.StartsWith(@"\\.\", StringComparison.Ordinal)
                || (path.StartsWith(@"\", StringComparison.Ordinal) && !path.StartsWith(@"\\", StringComparison.Ordinal)))
                throw new InvalidDataException("数据目录必须是完整的 Windows 路径：" + path);
            var full = Path.GetFullPath(path);
            return string.Equals(full, Path.GetPathRoot(full), StringComparison.OrdinalIgnoreCase)
                ? full : full.TrimEnd('\\', '/');
        }

        /// <summary>Compares directory boundaries without matching similarly prefixed siblings.</summary>
        public static bool IsWithin(string path, string root)
        {
            return Same(path, root) || Normalize(path).StartsWith(Normalize(root).TrimEnd('\\') + "\\", StringComparison.OrdinalIgnoreCase);
        }

        /// <summary>Compares canonical Windows paths case insensitively.</summary>
        private static bool Same(string left, string right)
        {
            return string.Equals(Normalize(left), Normalize(right), StringComparison.OrdinalIgnoreCase);
        }

        /// <summary>Rejects junctions and symbolic links anywhere above an explicitly selected root.</summary>
        private static void EnsureNoLinkedAncestor(string path)
        {
            for (var current = Normalize(path); current != null; current = Path.GetDirectoryName(current))
            {
                FileAttributes attributes;
                try { attributes = File.GetAttributes(current); }
                catch (FileNotFoundException) { continue; }
                catch (DirectoryNotFoundException) { continue; }
                if ((attributes & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("数据路径经过链接，请先处理该链接：" + current);
            }
        }

        /// <summary>Checks file locks without traversing nested junctions or symbolic links.</summary>
        private static void CheckTree(string directory)
        {
            foreach (var entry in new DirectoryInfo(directory).EnumerateFileSystemInfos())
            {
                if ((entry.Attributes & FileAttributes.ReparsePoint) != 0) continue;
                if (entry is DirectoryInfo) CheckTree(entry.FullName);
                else using (File.Open(entry.FullName, FileMode.Open, FileAccess.Read, FileShare.None)) { }
            }
        }

        /// <summary>Removes a tree without following links to data outside the approved roots.</summary>
        private static void DeleteTree(string directory, string bootstrapPath)
        {
            EnsureNoLinkedAncestor(directory);
            foreach (var entry in new DirectoryInfo(directory).EnumerateFileSystemInfos()
                .OrderBy(entry => IsWithin(bootstrapPath, entry.FullName) ? 1 : 0))
            {
                bool linked = (entry.Attributes & FileAttributes.ReparsePoint) != 0;
                if (entry is DirectoryInfo && !linked) DeleteTree(entry.FullName, bootstrapPath);
                else
                {
                    if (!linked) entry.Attributes &= ~FileAttributes.ReadOnly;
                    entry.Delete();
                }
            }
            Directory.Delete(directory);
        }

        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        private struct Credential
        {
            public uint Flags, Type;
            public string TargetName, Comment;
            public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
            public uint CredentialBlobSize;
            public IntPtr CredentialBlob;
            public uint Persist, AttributeCount;
            public IntPtr Attributes;
            public string TargetAlias, UserName;
        }

        /// <summary>Enumerates only Operit2 credentials owned by the current Windows user.</summary>
        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool CredEnumerate(string filter, uint flags, out uint count, out IntPtr credentials);

        /// <summary>Deletes one credential by its exact Windows target and type.</summary>
        [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        private static extern bool CredDelete(string target, uint type, uint flags);

        /// <summary>Releases the credential enumeration buffer.</summary>
        [DllImport("advapi32.dll")]
        private static extern void CredFree(IntPtr credentials);

        /// <summary>Clears app credentials without inspecting or logging their secret values.</summary>
        private static void DeleteCredentials()
        {
            if (!CredEnumerate("Operit2:*", 0, out var count, out var credentials))
            {
                int error = Marshal.GetLastWin32Error();
                if (error == 1168) return;
                throw new Win32Exception(error, "无法枚举 Operit2 凭据。");
            }
            try
            {
                for (int index = 0; index < count; index++)
                {
                    var credential = (Credential)Marshal.PtrToStructure(
                        Marshal.ReadIntPtr(credentials, index * IntPtr.Size), typeof(Credential));
                    if (credential.Type == 1 && credential.TargetName.StartsWith("Operit2:", StringComparison.Ordinal)
                        && !CredDelete(credential.TargetName, credential.Type, 0))
                        throw new Win32Exception(Marshal.GetLastWin32Error(), "无法删除 Operit2 凭据。");
                }
            }
            finally { CredFree(credentials); }
        }

        /// <summary>Removes shell entries only when their registered target belongs to this app copy.</summary>
        public static void RemoveShellEntries(string folder, string productCode)
        {
            RemoveShellEntries(folder, productCode, @"Software\Classes\operit2",
                Environment.GetFolderPath(Environment.SpecialFolder.Programs),
                Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory));
        }

        /// <summary>Accepts isolated shell locations for ownership tests without touching real app registrations.</summary>
        internal static void RemoveShellEntries(string folder, string productCode, string protocolKey, string programs, string desktop)
        {
            var executable = Path.Combine(Normalize(folder), "operit2.exe");
            using (var command = Registry.CurrentUser.OpenSubKey(protocolKey + @"\shell\open\command"))
            {
                var registered = command?.GetValue(null) as string;
                if (string.Equals(registered, "\"" + executable + "\" \"%1\"", StringComparison.OrdinalIgnoreCase))
                    Registry.CurrentUser.DeleteSubKeyTree(protocolKey, false);
            }
            var menu = Path.Combine(programs, "Operit2");
            var shellType = Type.GetTypeFromProgID("WScript.Shell", true);
            dynamic shell = Activator.CreateInstance(shellType);
            try
            {
                foreach (var path in new[] { Path.Combine(menu, "Operit2.lnk"),
                    Path.Combine(menu, "Operit2 Data Rescue.lnk"), Path.Combine(menu, "Operit2 Data Cleanup.lnk"),
                    Path.Combine(menu, "Operit2 Uninstall.lnk"),
                    Path.Combine(desktop, "Operit2.lnk") })
                {
                    if (!File.Exists(path)) continue;
                    dynamic shortcut = shell.CreateShortcut(path);
                    try
                    {
                        string target = shortcut.TargetPath;
                        string arguments = shortcut.Arguments;
                        if (string.Equals(target, executable, StringComparison.OrdinalIgnoreCase)
                            || string.Equals(target, Path.Combine(folder, "Operit2DataCleanup.exe"), StringComparison.OrdinalIgnoreCase)
                            || (string.Equals(target, Path.Combine(Environment.SystemDirectory, "msiexec.exe"), StringComparison.OrdinalIgnoreCase)
                                && arguments == UninstallArguments(productCode)))
                            File.Delete(path);
                    }
                    finally { Marshal.FinalReleaseComObject(shortcut); }
                }
            }
            finally { Marshal.FinalReleaseComObject(shell); }
            if (Directory.Exists(menu) && !Directory.EnumerateFileSystemEntries(menu).Any()) Directory.Delete(menu);
        }

        /// <summary>Creates shell shortcuts whose ownership can be checked again during removal.</summary>
        public static void RegisterShellEntries(string folder, string productCode)
        {
            RegisterShellEntries(folder, productCode, Environment.GetFolderPath(Environment.SpecialFolder.Programs),
                Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory));
        }

        /// <summary>Writes shortcuts to explicit directories for isolated ownership verification.</summary>
        internal static void RegisterShellEntries(string folder, string productCode, string programs, string desktop)
        {
            var menu = Path.Combine(programs, "Operit2");
            Directory.CreateDirectory(menu);
            dynamic shell = Activator.CreateInstance(Type.GetTypeFromProgID("WScript.Shell", true));
            try
            {
                WriteShortcut(shell, Path.Combine(menu, "Operit2.lnk"), Path.Combine(folder, "operit2.exe"), "", folder);
                WriteShortcut(shell, Path.Combine(menu, "Operit2 Data Rescue.lnk"), Path.Combine(folder, "operit2.exe"), "--data-rescue", folder);
                WriteShortcut(shell, Path.Combine(menu, "Operit2 Data Cleanup.lnk"), Path.Combine(folder, "Operit2DataCleanup.exe"), "", folder);
                WriteShortcut(shell, Path.Combine(menu, "Operit2 Uninstall.lnk"), Path.Combine(Environment.SystemDirectory, "msiexec.exe"),
                    UninstallArguments(productCode), folder);
                WriteShortcut(shell, Path.Combine(desktop, "Operit2.lnk"),
                    Path.Combine(folder, "operit2.exe"), "", folder);
            }
            finally { Marshal.FinalReleaseComObject(shell); }
        }

        /// <summary>Requests the complete uninstall UI even when the system application list uses basic UI.</summary>
        private static string UninstallArguments(string productCode)
        {
            return "/x \"" + Guid.Parse(productCode).ToString("B").ToUpperInvariant() + "\" /qf";
        }

        /// <summary>Writes one shortcut with an explicit target, argument list, and working directory.</summary>
        private static void WriteShortcut(dynamic shell, string path, string target, string arguments, string folder)
        {
            dynamic shortcut = shell.CreateShortcut(path);
            try
            {
                shortcut.TargetPath = target;
                shortcut.Arguments = arguments;
                shortcut.WorkingDirectory = folder;
                shortcut.IconLocation = Path.Combine(folder, "operit2.exe") + ",0";
                shortcut.Save();
            }
            finally { Marshal.FinalReleaseComObject(shortcut); }
        }
    }
}
