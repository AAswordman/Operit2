using System;
using System.IO;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;

namespace Operit.Installer
{
    public static class Program
    {
        /// <summary>Opens a standalone cleanup tool that does not require Flutter or an installed MSI.</summary>
        [STAThread]
        public static void Main()
        {
            var app = new Application();
            app.Run(new CleanupWindow());
        }
    }

    public sealed class CleanupWindow : Window
    {
        private readonly UserDataCleanup cleanup;
        private readonly TextBox details;
        private readonly Button remove;
        private readonly CheckBox consent;
        private CleanupPlan plan;
        private bool running;

        /// <summary>Lists resolved roots and requires explicit consent before enabling permanent deletion.</summary>
        public CleanupWindow()
        {
            Title = "Operit2 用户数据清理";
            Width = 760;
            Height = 640;
            MinWidth = 620;
            MinHeight = 520;
            WindowStartupLocation = WindowStartupLocation.CenterScreen;
            FontFamily = new System.Windows.Media.FontFamily("Microsoft YaHei UI");
            FontSize = 13;
            Background = System.Windows.Media.Brushes.White;
            var panel = new DockPanel { Margin = new Thickness(26), Background = System.Windows.Media.Brushes.White };
            Content = panel;
            var heading = new TextBlock { Text = "清理本机 Operit2 用户数据", FontSize = 23, Margin = new Thickness(0, 0, 0, 16) };
            DockPanel.SetDock(heading, Dock.Top);
            panel.Children.Add(heading);
            var actions = new StackPanel { Margin = new Thickness(0, 16, 0, 0) };
            DockPanel.SetDock(actions, Dock.Bottom);
            panel.Children.Add(actions);
            consent = new CheckBox { Content = "我已备份需要的数据，并理解这会影响本机所有 Operit2 副本。", Margin = new Thickness(0, 0, 0, 14) };
            actions.Children.Add(consent);
            remove = new Button { Content = "永久删除上述数据", Height = 42, IsEnabled = false,
                Background = new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(54, 97, 142)),
                Foreground = System.Windows.Media.Brushes.White };
            actions.Children.Add(remove);
            details = new TextBox { IsReadOnly = true, TextWrapping = TextWrapping.Wrap,
                VerticalScrollBarVisibility = ScrollBarVisibility.Auto, Padding = new Thickness(12) };
            panel.Children.Add(details);
            cleanup = new UserDataCleanup(AppDomain.CurrentDomain.BaseDirectory);
            try
            {
                plan = cleanup.CreatePlan();
                details.Text = plan.Describe() + "\n\n请先退出所有 Operit2 窗口和后台进程。\n" +
                    "此工具只清理数据；MSI 安装请通过系统设置卸载应用，压缩包版随后可删除程序目录。";
            }
            catch (Exception error) { details.Text = "无法确定安全的清理范围：\n" + error.Message; }
            consent.Checked += ConsentChanged;
            consent.Unchecked += ConsentChanged;
            remove.Click += DeleteData;
            Closing += PreventClosingWhileRunning;
        }

        /// <summary>Enables deletion only for a resolved plan with explicit user consent.</summary>
        private void ConsentChanged(object sender, RoutedEventArgs args)
        {
            remove.IsEnabled = plan != null && consent.IsChecked == true && !running;
        }

        /// <summary>Confirms irreversible deletion and keeps failures visible without claiming success.</summary>
        private async void DeleteData(object sender, RoutedEventArgs args)
        {
            if (MessageBox.Show(this, "永久删除列出的所有本机数据？此操作无法撤销。", "确认清理",
                MessageBoxButton.YesNo, MessageBoxImage.Warning, MessageBoxResult.No) != MessageBoxResult.Yes) return;
            running = true;
            remove.IsEnabled = false;
            consent.IsEnabled = false;
            try
            {
                await Task.Run(() => cleanup.DeleteData(plan));
                details.Text = "用户数据清理完成。其他设备和云端的数据未被修改。\n" +
                    "重新运行任一 Operit2 副本会重新创建本机数据。";
                consent.Visibility = Visibility.Collapsed;
                remove.Content = "清理完成";
                plan = null;
            }
            catch (Exception error)
            {
                details.Text = "清理未全部完成；已删除的数据无法恢复。\n" + error.Message + "\n\n" + plan.Describe();
                consent.IsEnabled = true;
            }
            finally { running = false; ConsentChanged(this, new RoutedEventArgs()); }
        }

        /// <summary>Keeps the process alive until its filesystem operation has finished.</summary>
        private void PreventClosingWhileRunning(object sender, System.ComponentModel.CancelEventArgs args)
        {
            args.Cancel = running;
        }
    }
}
