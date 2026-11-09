using System;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Web.Script.Serialization;
using Operit.Installer;
using Microsoft.Win32;

public static class CleanupTests
{
    private static int passed;

    /// <summary>Runs destructive cleanup only inside disposable fixtures, with credentials and process checks injected.</summary>
    public static int Main(string[] args)
    {
        try
        {
            Check("default and configured data deleted; unrelated siblings preserved", DeleteExpectedData);
            Check("configuration changes invalidate consent before deletion", RejectChangedPlan);
            Check("malformed bootstrap prevents any deletion", RejectBrokenConfig);
            Check("broad and relative roots are rejected", RejectUnsafeRoots);
            Check("ancestor roots are collapsed without matching sibling prefixes", CollapseRoots);
            Check("locked files prevent deletion of every root", RejectLockedFiles);
            Check("credential failures preserve all directories", PreserveDataOnCredentialFailure);
            Check("running runtime prevents deletion", RejectRunningRuntime);
            Check("linked roots are rejected and nested junction targets are preserved", JunctionSafety);
            Check("unconfirmed installations resolve only application-owned defaults", UnconfirmedPlan);
            Check("owned shell entries removed and another copy's entries preserved", ShellOwnership);
            if (args.Length == 1) MsiContractTests.Verify(Path.GetFullPath(args[0]));
            Console.WriteLine("Passed: " + passed);
            return 0;
        }
        catch (Exception error) { Console.Error.WriteLine(error); return 1; }
    }

    /// <summary>Handles a fresh installation before custom directories have been confirmed.</summary>
    private static void UnconfirmedPlan()
    {
        using (var f = new Fixture())
        {
            var withoutConfig = f.Cleanup().CreatePlan();
            f.Put(f.Bootstrap);
            File.WriteAllText(f.Bootstrap, "{\"confirmed\":false,\"runtimeRoot\":\"\",\"workspaceRoot\":\"\"}");
            Require(f.Cleanup().CreatePlan().Directories.SequenceEqual(withoutConfig.Directories));
        }
    }

    /// <summary>Exercises real Windows shortcuts and an isolated HKCU key for two competing app copies.</summary>
    private static void ShellOwnership()
    {
        using (var f = new Fixture())
        {
            var registryPath = @"Software\Operit\CleanupTests\" + Guid.NewGuid().ToString("N");
            var programs = Path.Combine(f.Root, "menus");
            var desktop = Path.Combine(f.Root, "desktop");
            Directory.CreateDirectory(desktop);
            var foreign = Path.Combine(f.Root, "other-copy");
            string productCode = Guid.NewGuid().ToString("B");
            try
            {
                UserDataCleanup.RegisterShellEntries(f.Install, productCode, programs, desktop);
                using (var key = Registry.CurrentUser.CreateSubKey(registryPath + @"\shell\open\command"))
                    key.SetValue(null, "\"" + Path.Combine(f.Install, "operit2.exe") + "\" \"%1\"");
                UserDataCleanup.RemoveShellEntries(f.Install, productCode, registryPath, programs, desktop);
                Require(!Directory.Exists(Path.Combine(programs, "Operit2")));
                Require(!File.Exists(Path.Combine(desktop, "Operit2.lnk")));
                using (var key = Registry.CurrentUser.OpenSubKey(registryPath)) Require(key == null);

                UserDataCleanup.RegisterShellEntries(foreign, Guid.NewGuid().ToString("B"), programs, desktop);
                using (var key = Registry.CurrentUser.CreateSubKey(registryPath + @"\shell\open\command"))
                    key.SetValue(null, "\"" + Path.Combine(foreign, "operit2.exe") + "\" \"%1\"");
                UserDataCleanup.RemoveShellEntries(f.Install, productCode, registryPath, programs, desktop);
                Require(Directory.GetFiles(Path.Combine(programs, "Operit2")).Length == 4);
                Require(File.Exists(Path.Combine(desktop, "Operit2.lnk")));
                using (var key = Registry.CurrentUser.OpenSubKey(registryPath)) Require(key != null);
            }
            finally { Registry.CurrentUser.DeleteSubKeyTree(registryPath, false); }
        }
    }

    /// <summary>Runs one isolated scenario and prints its behavioral outcome.</summary>
    private static void Check(string name, Action test)
    {
        test();
        passed++;
        Console.WriteLine("PASS " + name);
    }

    /// <summary>Requires a failed operation rather than silently accepting an unsafe request.</summary>
    private static void Reject(Action action)
    {
        try { action(); }
        catch (Exception) { return; }
        throw new Exception("Expected operation to fail.");
    }

    /// <summary>Fails the test when an expected invariant is false.</summary>
    private static void Require(bool condition)
    {
        if (!condition) throw new Exception("Invariant failed.");
    }

    /// <summary>Exercises real deletion of all expected roots without touching Windows credentials.</summary>
    private static void DeleteExpectedData()
    {
        using (var f = new Fixture())
        {
            f.Configure(f.Runtime, f.Workspaces);
            var cleanup = f.Cleanup();
            var plan = cleanup.CreatePlan();
            foreach (var root in plan.Directories) f.Put(Path.Combine(root, "identities", "sample", "data.txt"));
            f.Put(Path.Combine(f.Root, "unrelated", "keep.txt"));
            cleanup.DeleteData(plan);
            Require(plan.Directories.All(path => !Directory.Exists(path)));
            Require(File.Exists(Path.Combine(f.Root, "unrelated", "keep.txt")));
            Require(f.CredentialsCleared == 1);
        }
    }

    /// <summary>Prevents consent from being reused after a runtime changes its data directories.</summary>
    private static void RejectChangedPlan()
    {
        using (var f = new Fixture())
        {
            f.Configure(f.Runtime, f.Workspaces);
            f.Put(Path.Combine(f.Runtime, "keep.txt"));
            var cleanup = f.Cleanup();
            var plan = cleanup.CreatePlan();
            f.Configure(Path.Combine(f.Root, "other"), f.Workspaces);
            Reject(() => cleanup.DeleteData(plan));
            Require(File.Exists(Path.Combine(f.Runtime, "keep.txt")) && f.CredentialsCleared == 0);
        }
    }

    /// <summary>Blocks cleanup when bootstrap data cannot reliably identify custom roots.</summary>
    private static void RejectBrokenConfig()
    {
        using (var f = new Fixture())
        {
            f.Put(f.Bootstrap);
            foreach (var invalid in new[] { "{", "null", "{}", "{\"confirmed\":true}" })
            {
                File.WriteAllText(f.Bootstrap, invalid);
                Reject(() => f.Cleanup().CreatePlan());
                Require(File.Exists(f.Bootstrap));
            }
        }
    }

    /// <summary>Protects disk roots, known user folders, install directories, and ambiguous paths.</summary>
    private static void RejectUnsafeRoots()
    {
        using (var f = new Fixture())
        {
            foreach (var root in new[] { Path.GetPathRoot(f.Root), f.Root, f.Roaming, f.Install,
                Path.Combine(f.Root, "documents"), "relative", "C:relative", @"\root-relative", @"\\?\C:\data" })
            {
                f.Configure(root, f.Workspaces);
                Reject(() => f.Cleanup().CreatePlan());
            }
            Require(!UserDataCleanup.IsWithin(f.Runtime + "-other", f.Runtime));
        }
    }

    /// <summary>Deletes overlapping roots once and keeps the bootstrap directory until the end.</summary>
    private static void CollapseRoots()
    {
        using (var f = new Fixture())
        {
            var app = Path.Combine(f.Roaming, "Operit2");
            f.Configure(Path.Combine(app, "runtime"), Path.Combine(app, "workspaces"));
            var plan = f.Cleanup().CreatePlan();
            Require(plan.Directories.Count(path => UserDataCleanup.IsWithin(path, app)) == 1);
            Require(plan.Directories.Last() == app);
        }
    }

    /// <summary>Checks all file locks before clearing credentials or deleting any directory.</summary>
    private static void RejectLockedFiles()
    {
        using (var f = new Fixture())
        {
            f.Configure(f.Runtime, f.Workspaces);
            string locked = Path.Combine(f.Workspaces, "locked.db");
            f.Put(locked);
            f.Put(Path.Combine(f.Runtime, "keep.txt"));
            var cleanup = f.Cleanup();
            var plan = cleanup.CreatePlan();
            using (File.Open(locked, FileMode.Open, FileAccess.ReadWrite, FileShare.None))
                Reject(() => cleanup.DeleteData(plan));
            Require(File.Exists(Path.Combine(f.Runtime, "keep.txt")) && f.CredentialsCleared == 0);
        }
    }

    /// <summary>Stops before file deletion when Windows credential cleanup reports an error.</summary>
    private static void PreserveDataOnCredentialFailure()
    {
        using (var f = new Fixture())
        {
            f.Configure(f.Runtime, f.Workspaces);
            f.Put(Path.Combine(f.Runtime, "keep.txt"));
            var cleanup = f.Cleanup(() => { throw new IOException("credential fixture failure"); });
            Reject(() => cleanup.DeleteData(cleanup.CreatePlan()));
            Require(File.Exists(Path.Combine(f.Runtime, "keep.txt")) && File.Exists(f.Bootstrap));
        }
    }

    /// <summary>Rejects data deletion before any mutations when the runtime is still active.</summary>
    private static void RejectRunningRuntime()
    {
        using (var f = new Fixture())
        {
            f.Configure(f.Runtime, f.Workspaces);
            var cleanup = f.Cleanup(null, () => { throw new InvalidOperationException("runtime fixture active"); });
            Reject(() => cleanup.DeleteData(cleanup.CreatePlan()));
            Require(File.Exists(f.Bootstrap) && f.CredentialsCleared == 0);
        }
    }

    /// <summary>Verifies real NTFS junction behavior without following links outside approved data.</summary>
    private static void JunctionSafety()
    {
        using (var f = new Fixture())
        {
            string outside = Path.Combine(f.Root, "outside");
            string link = Path.Combine(f.Root, "linked-root");
            f.Put(Path.Combine(outside, "keep.txt"));
            CreateJunction(link, outside);
            try
            {
                f.Configure(link, f.Workspaces);
                Reject(() => f.Cleanup().CreatePlan());
            }
            finally { Directory.Delete(link); }
            f.Configure(f.Runtime, f.Workspaces);
            Directory.CreateDirectory(f.Runtime);
            CreateJunction(Path.Combine(f.Runtime, "linked-child"), outside);
            var cleanup = f.Cleanup();
            cleanup.DeleteData(cleanup.CreatePlan());
            Require(File.Exists(Path.Combine(outside, "keep.txt")));
        }
    }

    /// <summary>Creates a test junction using a fixed command and explicit fixture paths.</summary>
    private static void CreateJunction(string link, string target)
    {
        var start = new ProcessStartInfo("cmd.exe", "/c mklink /J \"" + link + "\" \"" + target + "\"")
            { UseShellExecute = false, CreateNoWindow = true, RedirectStandardOutput = true, RedirectStandardError = true };
        using (var process = Process.Start(start))
        {
            process.WaitForExit();
            Require(process.ExitCode == 0);
        }
    }

    private sealed class Fixture : IDisposable
    {
        public readonly string Root = Path.Combine(Path.GetTempPath(), "operit-cleanup-tests-" + Guid.NewGuid().ToString("N"));
        public string Roaming => Path.Combine(Root, "roaming");
        public string Local => Path.Combine(Root, "local");
        public string Install => Path.Combine(Root, "program");
        public string Runtime => Path.Combine(Root, "custom-runtime");
        public string Workspaces => Path.Combine(Root, "custom-workspaces");
        public string Bootstrap => Path.Combine(Roaming, "Operit2", "client", "link", "local_runtime_storage.json");
        public int CredentialsCleared;

        /// <summary>Creates only disposable storage owned by this test fixture.</summary>
        public Fixture() { Directory.CreateDirectory(Root); }

        /// <summary>Builds the cleaner with isolated credentials and an explicit process guard.</summary>
        public UserDataCleanup Cleanup(Action credentials = null, Action stopped = null)
        {
            return new UserDataCleanup(Roaming, Local, Install, new[] { Root, Path.Combine(Root, "documents") },
                credentials ?? (() => CredentialsCleared++), stopped ?? (() => { }));
        }

        /// <summary>Writes the same bootstrap fields used by the Flutter client.</summary>
        public void Configure(string runtime, string workspace)
        {
            Put(Bootstrap);
            File.WriteAllText(Bootstrap, new JavaScriptSerializer().Serialize(new {
                confirmed = true, runtimeRoot = runtime, workspaceRoot = workspace }));
        }

        /// <summary>Creates a small data file under the fixture's explicit root.</summary>
        public void Put(string path)
        {
            Require(UserDataCleanup.IsWithin(path, Root));
            Directory.CreateDirectory(Path.GetDirectoryName(path));
            File.WriteAllText(path, "fixture data");
        }

        /// <summary>Checks fixture ownership before recursively removing its disposable test directory.</summary>
        public void Dispose()
        {
            Require(Path.GetDirectoryName(Root).TrimEnd('\\') == Path.GetTempPath().TrimEnd('\\'));
            if (Directory.Exists(Root)) Directory.Delete(Root, true);
        }
    }
}
