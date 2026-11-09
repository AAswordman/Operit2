using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Windows;
using System.Windows.Input;

namespace Operit.Installer
{
    public enum SetupOperation { Install, Repair, Remove }

    public partial class SetupWindow : Window
    {
        private readonly ManualResetEvent start;
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
        public SetupWindow(string version, string folder, bool installed, bool removing, ManualResetEvent start)
        {
            InitializeComponent();
            this.start = start;
            InstallFolder = folder;
            LocationText.Text = folder;
            VersionText.Text = version;
            Operation = removing ? SetupOperation.Remove : installed ? SetupOperation.Repair : SetupOperation.Install;
            if (installed || removing)
            {
                Heading.Text = removing ? "卸载 Operit2" : "管理 Operit2";
                Description.Text = removing ? "移除应用，保留你的聊天记录和个人设置。" : "修复应用文件，或从电脑中移除应用。";
                MainButton.Content = removing ? "确认卸载" : "修复应用";
                SecondaryButton.Content = removing ? "取消" : "卸载应用";
                LocationText.IsEnabled = false;
                ChangeLocationButton.Visibility = Visibility.Collapsed;
                InstallHint.Text = removing
                    ? "默认保留数据。勾选后会删除聊天、设置、插件、工作区和密钥，\n并影响本机共用这些数据的其他副本。"
                    : "修复会恢复应用所需的文件。\n卸载会保留你的聊天记录和个人设置。";
                DeleteUserData.Visibility = removing ? Visibility.Visible : Visibility.Collapsed;
                CleanupDetailsButton.Visibility = removing ? Visibility.Visible : Visibility.Collapsed;
            }
        }

        /// <summary>Starts an MSI operation or opens the successfully installed application.</summary>
        private void PrimaryAction(object sender, RoutedEventArgs e)
        {
            if (finished)
            {
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
                    cleanup.ValidatePlan(DataCleanupPlan);
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
            Heading.Text = Operation == SetupOperation.Remove ? "正在卸载…" : Operation == SetupOperation.Repair ? "正在修复…" : "正在安装…";
            Description.Text = "很快就好，请保持安装窗口打开。";
            MainButton.IsEnabled = false;
            MainButton.Content = "正在处理…";
            SecondaryButton.Content = "取消";
            start.Set();
        }

        /// <summary>Selects uninstall from the maintenance screen or closes the wizard.</summary>
        private void SecondaryAction(object sender, RoutedEventArgs e)
        {
            if (!running && !finished && Operation == SetupOperation.Repair)
            {
                Operation = SetupOperation.Remove;
                Heading.Text = "卸载 Operit2";
                Description.Text = "移除应用，保留你的聊天记录和个人设置。";
                MainButton.Content = "确认卸载";
                SecondaryButton.Content = "取消";
                DeleteUserData.Visibility = Visibility.Visible;
                CleanupDetailsButton.Visibility = Visibility.Visible;
                InstallHint.Text = "默认保留数据。勾选后会删除聊天、设置、插件、工作区和密钥，\n并影响本机共用这些数据的其他副本。";
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
        }

        /// <summary>Shows a success, cancellation, or error result after the transaction ends.</summary>
        public void Complete(bool success, string error)
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
                Heading.Text = Operation == SetupOperation.Remove ? "卸载完成" : "一切就绪";
                Description.Text = Operation == SetupOperation.Remove ? "期待与你再次相见。" : "你的 AI 工作空间，已准备就绪。";
                ResultText.Text = Operation == SetupOperation.Remove
                    ? DeleteData ? "本机用户数据和 Operit2 凭据已删除。其他设备与云端的数据未被修改。" : "聊天记录和个人设置已保留。"
                    : "你可以从桌面或开始菜单打开 Operit2。\n\n让下一个想法，从这里开始。";
                MainButton.Content = Operation == SetupOperation.Remove ? "完成" : "打开 Operit2";
            }
            else
            {
                Heading.Text = CancelRequested ? "操作已取消" : "未能完成操作";
                Description.Text = CancelRequested ? "安装程序已停止本次操作。" : "请检查下面的原因，然后重新运行安装程序。";
                ResultText.Text = error;
                MainButton.Content = "关闭";
                SecondaryButton.Visibility = Visibility.Collapsed;
            }
        }
    }
}
