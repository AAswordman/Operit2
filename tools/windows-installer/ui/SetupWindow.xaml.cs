using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;

namespace Operit.Installer
{
    public partial class SetupWindow : Window
    {
        private readonly ManualResetEvent start;
        private readonly bool installed;
        private readonly SetupDiagnostics diagnostics;
        private bool running;
        private bool finished;
        private bool succeeded;
        private bool finalizing;
        public bool CancelRequested { get; private set; }
        public string InstallFolder { get; private set; }
        public SetupOperation Operation { get; private set; }
        public CleanupPlan DataCleanupPlan { get; private set; }
        public bool DeleteData => Operation == SetupOperation.Remove && DeleteUserData.IsChecked == true;

        /// <summary>Creates the custom install, maintenance, or uninstall screen.</summary>
        public SetupWindow(string version, string folder, SetupOperation operation, bool installed, ManualResetEvent start,
            SetupDiagnostics diagnostics)
        {
            InitializeComponent();
            this.start = start;
            this.installed = installed;
            this.diagnostics = diagnostics;
            InstallFolder = folder;
            LocationText.Text = folder;
            VersionText.Text = version;
            Operation = operation;
            MsiLogButton.IsEnabled = !string.IsNullOrWhiteSpace(diagnostics.MsiLogPath);
            if (operation == SetupOperation.Update)
            {
                Heading.Text = "更新 Operit2";
                Description.Text = "发现已安装的旧版本，更新后即可使用新版本。";
                MainButton.Content = "更新应用";
                SecondaryButton.Content = "稍后再说";
                LocationText.IsEnabled = false;
                ChangeLocationButton.Visibility = Visibility.Collapsed;
                InstallHint.Text = "更新将沿用当前安装位置。\n你的聊天记录、个人设置、插件和工作区会保留。";
            }
            else if (operation == SetupOperation.Repair || operation == SetupOperation.Remove)
            {
                bool removing = operation == SetupOperation.Remove;
                Heading.Text = removing ? "卸载 Operit2" : "管理 Operit2";
                Description.Text = removing ? "移除应用，保留你的聊天记录和个人设置。"
                    : installed ? "修复应用文件，或从电脑中移除应用。" : "已安装相同版本，可修复应用文件。";
                MainButton.Content = removing ? "确认卸载" : "修复应用";
                SecondaryButton.Content = removing ? "取消" : installed ? "卸载应用" : "稍后再说";
                LocationText.IsEnabled = false;
                ChangeLocationButton.Visibility = Visibility.Collapsed;
                InstallHint.Text = removing
                    ? "默认保留数据。勾选后会删除聊天、设置、插件、工作区和密钥，\n并影响本机共用这些数据的其他副本。"
                    : installed ? "修复会恢复应用所需的文件。\n卸载会保留你的聊天记录和个人设置。"
                    : "修复会恢复应用所需的文件。\n你的聊天记录和个人设置会保留。";
                DeleteUserData.Visibility = removing ? Visibility.Visible : Visibility.Collapsed;
                CleanupDetailsButton.Visibility = removing ? Visibility.Visible : Visibility.Collapsed;
            }
            InstallHint.Text += "\n开始后会自动退出此安装位置的 Operit2。";
        }

        /// <summary>Starts an MSI operation or opens the successfully installed application.</summary>
        private void PrimaryAction(object sender, RoutedEventArgs e)
        {
            if (finished)
            {
                if (!succeeded && (!CancelRequested || diagnostics.HasErrors))
                {
                    CopyDetails(sender, e);
                    return;
                }
                if (succeeded && Operation != SetupOperation.Remove)
                    Process.Start(new ProcessStartInfo(Path.Combine(InstallFolder, "operit2.exe")) { UseShellExecute = true });
                Close();
                return;
            }
            try
            {
                if (!Path.IsPathRooted(LocationText.Text))
                    throw new ArgumentException("请输入完整的安装路径，例如 D:\\Apps\\Operit2。");
                InstallFolder = Path.GetFullPath(LocationText.Text);
                if (InstallFolder.TrimEnd('\\').Length < 4)
                    throw new ArgumentException("请选择一个应用文件夹，不要直接安装到磁盘根目录。");
                if (DeleteData)
                {
                    var cleanup = new UserDataCleanup(InstallFolder);
                    DataCleanupPlan = cleanup.CreatePlan();
                    if (MessageBox.Show(this, DataCleanupPlan.Describe(), "确认永久删除本机用户数据",
                        MessageBoxButton.YesNo, MessageBoxImage.Warning, MessageBoxResult.No) != MessageBoxResult.Yes) return;
                }
            }
            catch (Exception error)
            {
                Description.Text = error.Message;
                return;
            }
            running = true;
            LocationPanel.Visibility = Visibility.Collapsed;
            ProgressPanel.Visibility = Visibility.Visible;
            Heading.Text = Operation == SetupOperation.Remove ? "正在卸载…"
                : Operation == SetupOperation.Update ? "正在更新…"
                : Operation == SetupOperation.Repair ? "正在修复…" : "正在安装…";
            Description.Text = "很快就好，请保持安装窗口打开。";
            MainButton.IsEnabled = false;
            MainButton.Content = "正在处理…";
            SecondaryButton.Content = "取消";
            start.Set();
        }

        /// <summary>Selects uninstall from the maintenance screen or closes the wizard.</summary>
        private void SecondaryAction(object sender, RoutedEventArgs e)
        {
            if (!running && !finished && installed && Operation == SetupOperation.Repair)
            {
                Operation = SetupOperation.Remove;
                Heading.Text = "卸载 Operit2";
                Description.Text = "移除应用，保留你的聊天记录和个人设置。";
                MainButton.Content = "确认卸载";
                SecondaryButton.Content = "取消";
                DeleteUserData.Visibility = Visibility.Visible;
                CleanupDetailsButton.Visibility = Visibility.Visible;
                InstallHint.Text = "默认保留数据。勾选后会删除聊天、设置、插件、工作区和密钥，\n并影响本机共用这些数据的其他副本。\n开始后会自动退出此安装位置的 Operit2。";
                return;
            }
            Close();
        }

        /// <summary>Explains the shared data scope when permanent removal is selected.</summary>
        private void CleanupChoiceChanged(object sender, RoutedEventArgs args)
        {
            Description.Text = DeleteUserData.IsChecked == true
                ? "移除应用，并永久清理本机共用的用户数据。" : "移除应用，保留你的聊天记录和个人设置。";
        }

        /// <summary>Displays configured roots without changing or deleting any data.</summary>
        private void ShowCleanupDetails(object sender, RoutedEventArgs args)
        {
            try { MessageBox.Show(this, new UserDataCleanup(InstallFolder).CreatePlan().Describe(), "用户数据清理范围"); }
            catch (Exception error) { MessageBox.Show(this, error.Message, "无法确定清理范围"); }
        }

        /// <summary>Turns an in-progress close into an MSI cancellation request.</summary>
        private void OnClosing(object sender, CancelEventArgs e)
        {
            if (!running) return;
            e.Cancel = true;
            if (finalizing) return;
            CancelRequested = true;
            ProgressText.Text = "正在取消，请稍候…";
            SecondaryButton.IsEnabled = false;
            CloseButton.IsEnabled = false;
        }

        /// <summary>Disables cancellation while completing work after the MSI transaction has committed.</summary>
        public void BeginFinalization()
        {
            finalizing = true;
            SecondaryButton.IsEnabled = false;
            CloseButton.IsEnabled = false;
            SecondaryButton.Content = "正在完成…";
        }

        /// <summary>Enables direct editing of the installation directory.</summary>
        private void EditLocation(object sender, RoutedEventArgs e)
        {
            if (Operation != SetupOperation.Install) return;
            LocationText.IsReadOnly = false;
            LocationText.Focus();
            LocationText.SelectAll();
        }

        /// <summary>Moves the frameless window using the custom title bar.</summary>
        private void DragTitleBar(object sender, MouseButtonEventArgs e)
        {
            if (e.ChangedButton == MouseButton.Left) DragMove();
        }

        /// <summary>Minimizes the custom window without showing a native title bar.</summary>
        private void Minimize(object sender, RoutedEventArgs e) { WindowState = WindowState.Minimized; }

        /// <summary>Routes the custom close button through cancellation handling.</summary>
        private void RequestClose(object sender, RoutedEventArgs e) { Close(); }

        /// <summary>Updates the progress display with MSI's reported work.</summary>
        public void ReportProgress(double value, string action)
        {
            Progress.Value = Math.Max(0, Math.Min(99, value));
            PercentText.Text = Math.Round(Progress.Value) + "%";
            if (action != null && !CancelRequested) ProgressText.Text = action;
            if (diagnostics.LoggingError != null)
            {
                Description.Text = diagnostics.LoggingError;
                CopyDetailsButton.Visibility = Visibility.Visible;
            }
        }

        /// <summary>Shows errors immediately while leaving Windows Installer time to stop or roll back.</summary>
        public void ShowFailure()
        {
            ExpandDiagnostics();
            Description.Text = "正在停止安装，请稍候…";
        }

        /// <summary>Shows a short explanation and keeps raw diagnostics in a collapsed details section.</summary>
        private void ExpandDiagnostics()
        {
            BrandPanel.Visibility = Visibility.Collapsed;
            Grid.SetColumn(ContentPanel, 0);
            Grid.SetColumnSpan(ContentPanel, 2);
            Width = Math.Min(900, SystemParameters.WorkArea.Width - 24);
            Height = Math.Min(TechnicalDetails.IsExpanded ? 800 : 620, SystemParameters.WorkArea.Height - 24);
            Left = SystemParameters.WorkArea.Left + (SystemParameters.WorkArea.Width - Width) / 2;
            Top = SystemParameters.WorkArea.Top + (SystemParameters.WorkArea.Height - Height) / 2;
            LocationPanel.Visibility = Visibility.Collapsed;
            ProgressPanel.Visibility = Visibility.Collapsed;
            ResultText.Visibility = Visibility.Collapsed;
            FailurePanel.Visibility = Visibility.Visible;
            TechnicalDetails.Visibility = Visibility.Visible;
            var problem = diagnostics.Problem();
            Heading.Text = problem.Title;
            ProblemReason.Text = problem.Reason;
            ProblemSteps.Text = problem.Steps;
            ErrorDetailsText.Text = diagnostics.Describe();
            DiagnosticsPanel.Visibility = Visibility.Visible;
            ShowDiagnosticsButton.Visibility = Visibility.Collapsed;
            CopyDetailsButton.Visibility = Visibility.Visible;
        }

        /// <summary>Gives expanded technical details more space without enlarging the default error screen.</summary>
        private void ResizeTechnicalDetails(object sender, RoutedEventArgs e)
        {
            Height = Math.Min(TechnicalDetails.IsExpanded ? 800 : 620, SystemParameters.WorkArea.Height - 24);
            Top = SystemParameters.WorkArea.Top + (SystemParameters.WorkArea.Height - Height) / 2;
        }

        /// <summary>Opens the diagnostic report at any point without changing the MSI operation.</summary>
        private void ShowDiagnostics(object sender, RoutedEventArgs e)
        {
            var report = new Window
            {
                Title = "Operit2 安装日志与诊断", Owner = this, Width = Math.Min(850, SystemParameters.WorkArea.Width - 40),
                Height = Math.Min(680, SystemParameters.WorkArea.Height - 40), WindowStartupLocation = WindowStartupLocation.CenterOwner,
                Content = new TextBox
                {
                    Text = diagnostics.Describe(), IsReadOnly = true, TextWrapping = TextWrapping.Wrap,
                    VerticalScrollBarVisibility = ScrollBarVisibility.Auto,
                    Margin = new Thickness(16), Padding = new Thickness(12)
                }
            };
            report.Show();
        }

        /// <summary>Copies the complete diagnostic report and reports clipboard access failures.</summary>
        private void CopyDetails(object sender, RoutedEventArgs e)
        {
            try
            {
                Clipboard.SetText(diagnostics.Describe());
                if (finished) MainButton.Content = "已复制错误信息";
                else CopyDetailsButton.Content = "已复制错误信息";
            }
            catch (Exception error) { MessageBox.Show(this, "无法复制到剪贴板：" + error.Message + "\n可选中详情文本后按 Ctrl+C，或打开诊断日志。", "复制失败"); }
        }

        /// <summary>Opens the diagnostic file in the system-associated viewer.</summary>
        private void OpenDiagnosticLog(object sender, RoutedEventArgs e) { OpenDiagnosticPath(diagnostics.LogPath); }

        /// <summary>Opens the exact verbose MSI log supplied by Windows Installer.</summary>
        private void OpenMsiLog(object sender, RoutedEventArgs e) { OpenDiagnosticPath(diagnostics.MsiLogPath); }

        /// <summary>Opens the directory containing this operation's persistent diagnostic log.</summary>
        private void OpenLogFolder(object sender, RoutedEventArgs e) { OpenDiagnosticPath(Path.GetDirectoryName(diagnostics.LogPath)); }

        /// <summary>Reports file-association, missing-file, and permission errors without hiding them.</summary>
        private void OpenDiagnosticPath(string path)
        {
            try { Process.Start(new ProcessStartInfo(path) { UseShellExecute = true }); }
            catch (Exception error) { MessageBox.Show(this, "无法打开：" + path + "\n" + error.Message + "\n可以复制此路径到文件资源管理器中查看。", "无法打开日志", MessageBoxButton.OK, MessageBoxImage.Error); }
        }

        /// <summary>Shows a success, cancellation, or error result after the transaction ends.</summary>
        public void Complete(bool success)
        {
            running = false;
            finished = true;
            succeeded = success;
            CloseButton.IsEnabled = true;
            ProgressPanel.Visibility = Visibility.Collapsed;
            ResultText.Visibility = Visibility.Visible;
            MainButton.IsEnabled = true;
            SecondaryButton.IsEnabled = true;
            SecondaryButton.Content = "完成";
            if (success)
            {
                Heading.Text = Operation == SetupOperation.Remove ? "卸载完成"
                    : Operation == SetupOperation.Update ? "更新完成" : "一切就绪";
                Description.Text = Operation == SetupOperation.Remove ? "期待与你再次相见。" : "你的 AI 工作空间，已准备就绪。";
                ResultText.Text = Operation == SetupOperation.Remove
                    ? DeleteData ? "本机用户数据和 Operit2 凭据已删除。其他设备与云端的数据未被修改。" : "聊天记录和个人设置已保留。"
                    : Operation == SetupOperation.Update ? "Operit2 已更新到新版本。\n你的聊天记录和个人设置已保留。"
                    : "你可以从桌面或开始菜单打开 Operit2。\n\n让下一个想法，从这里开始。";
                MainButton.Content = Operation == SetupOperation.Remove ? "完成" : "打开 Operit2";
            }
            else
            {
                bool canceled = CancelRequested && !diagnostics.HasErrors;
                ExpandDiagnostics();
                if (canceled)
                {
                    Heading.Text = "操作已取消";
                    FailurePanel.Visibility = Visibility.Collapsed;
                }
                Description.Text = canceled ? "安装程序已停止本次操作。" : "请按下面的提示处理。";
                Description.Visibility = canceled ? Visibility.Visible : Visibility.Collapsed;
                MainButton.Content = canceled ? "关闭" : "复制错误信息";
                CopyDetailsButton.Visibility = Visibility.Collapsed;
                SecondaryButton.Content = "关闭";
                SecondaryButton.Visibility = canceled ? Visibility.Collapsed : Visibility.Visible;
            }
        }
    }
}
