using System;
using System.Runtime.InteropServices;
using System.Text;

public static class MsiContractTests
{
    /// <summary>Checks compiled MSI conditions and sequencing without installing or executing custom actions.</summary>
    public static void Verify(string path)
    {
        RequireSuccess(MsiOpenPackageEx(path, 1, out var session));
        try
        {
            uint database = MsiGetActiveDatabase(session);
            try
            {
                RequireSuccess(MsiDatabaseOpenView(database,
                    "SELECT `Value` FROM `Property` WHERE `Property`='MsiLogging'", out var logView));
                try
                {
                    RequireSuccess(MsiViewExecute(logView, 0));
                    RequireSuccess(MsiViewFetch(logView, out var logRecord));
                    try
                    {
                        var logging = new StringBuilder(128);
                        uint length = 128;
                        RequireSuccess(MsiRecordGetString(logRecord, 1, logging, ref length));
                        Require(logging.ToString() == "voicewarmup");
                    }
                    finally { MsiCloseHandle(logRecord); }
                }
                finally { MsiCloseHandle(logView); }
                var prepare = ReadAction(database, "OperitPrepareRemoval");
                var close = ReadAction(database, "OperitCloseApplications");
                var validate = ReadAction(database, "InstallValidate");
                var finish = ReadAction(database, "OperitFinishRemoval");
                var complete = ReadAction(database, "OperitInstallCompleted");
                var initialize = ReadAction(database, "InstallInitialize");
                var finalize = ReadAction(database, "InstallFinalize");
                Require(prepare.Sequence < initialize.Sequence && finish.Sequence > finalize.Sequence && complete.Sequence > finish.Sequence);
                Require(close.Sequence < validate.Sequence);
                Evaluate(session, close.Condition, "1", "", "", true);
                Evaluate(session, close.Condition, "1", "ALL", "", true);
                Evaluate(session, close.Condition, "", "", "", true);
                Evaluate(session, close.Condition, "1", "ALL", "{old-product}", false);
                foreach (var condition in new[] { prepare.Condition, finish.Condition })
                {
                    Evaluate(session, condition, "1", "ALL", "", true);
                    Evaluate(session, condition, "1", "ALL", "{old-product}", false);
                    Evaluate(session, condition, "1", "", "", false);
                    Evaluate(session, condition, "", "", "", false);
                    Evaluate(session, condition, "", "ALL", "", false);
                    // An explicit command line cleanup opt-in must never bypass the upgrade guard.
                    RequireSuccess(MsiSetProperty(session, "OPERIT_DELETE_USER_DATA", "1"));
                    Evaluate(session, condition, "1", "ALL", "{old-product}", false);
                }
                Console.WriteLine("PASS compiled MSI verbose logging, uninstall, upgrade, repair, and post-commit conditions");
            }
            finally { MsiCloseHandle(database); }
        }
        finally { MsiCloseHandle(session); }
    }

    private sealed class ActionRow
    {
        public string Condition;
        public int Sequence;
    }

    /// <summary>Reads the compiled execution table rather than relying on authoring XML.</summary>
    private static ActionRow ReadAction(uint database, string name)
    {
        RequireSuccess(MsiDatabaseOpenView(database,
            "SELECT `Condition`, `Sequence` FROM `InstallExecuteSequence` WHERE `Action`='" + name + "'", out var view));
        try
        {
            RequireSuccess(MsiViewExecute(view, 0));
            RequireSuccess(MsiViewFetch(view, out var record));
            try
            {
                var condition = new StringBuilder(2048);
                uint size = 2048;
                RequireSuccess(MsiRecordGetString(record, 1, condition, ref size));
                return new ActionRow { Condition = condition.ToString(), Sequence = MsiRecordGetInteger(record, 2) };
            }
            finally { MsiCloseHandle(record); }
        }
        finally { MsiCloseHandle(view); }
    }

    /// <summary>Evaluates lifecycle scenarios in an unopened-for-execution Windows Installer session.</summary>
    private static void Evaluate(uint session, string condition, string installed, string remove, string upgrading, bool expected)
    {
        RequireSuccess(MsiSetProperty(session, "Installed", installed));
        RequireSuccess(MsiSetProperty(session, "REMOVE", remove));
        RequireSuccess(MsiSetProperty(session, "UPGRADINGPRODUCTCODE", upgrading));
        Require(MsiEvaluateCondition(session, condition) == (expected ? 1 : 0));
    }

    /// <summary>Fails immediately on MSI API errors.</summary>
    private static void RequireSuccess(uint error)
    {
        if (error != 0) throw new InvalidOperationException("MSI API error: " + error);
    }

    /// <summary>Requires the compiled installation contract to match the expected lifecycle behavior.</summary>
    private static void Require(bool condition)
    {
        if (!condition) throw new InvalidOperationException("MSI lifecycle contract failed.");
    }

    /// <summary>Opens an MSI for condition inspection without executing an installation.</summary>
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    private static extern uint MsiOpenPackageEx(string path, uint options, out uint session);
    /// <summary>Retrieves the session's active database handle.</summary>
    [DllImport("msi.dll")]
    private static extern uint MsiGetActiveDatabase(uint session);
    /// <summary>Opens a read-only query against an MSI database.</summary>
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    private static extern uint MsiDatabaseOpenView(uint database, string query, out uint view);
    /// <summary>Executes a database query without running installer actions.</summary>
    [DllImport("msi.dll")]
    private static extern uint MsiViewExecute(uint view, uint record);
    /// <summary>Fetches a compiled table row.</summary>
    [DllImport("msi.dll")]
    private static extern uint MsiViewFetch(uint view, out uint record);
    /// <summary>Reads a Unicode record field.</summary>
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    private static extern uint MsiRecordGetString(uint record, uint field, StringBuilder value, ref uint size);
    /// <summary>Reads a numeric record field.</summary>
    [DllImport("msi.dll")]
    private static extern int MsiRecordGetInteger(uint record, uint field);
    /// <summary>Sets scenario properties in the inspection session.</summary>
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    private static extern uint MsiSetProperty(uint session, string name, string value);
    /// <summary>Evaluates a condition without running a custom action.</summary>
    [DllImport("msi.dll", CharSet = CharSet.Unicode)]
    private static extern int MsiEvaluateCondition(uint session, string condition);
    /// <summary>Closes one MSI inspection handle.</summary>
    [DllImport("msi.dll")]
    private static extern uint MsiCloseHandle(uint handle);
}
