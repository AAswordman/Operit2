using System;
using System.Web.Script.Serialization;
using WixToolset.Dtf.WindowsInstaller;

namespace Operit.Installer
{
    public static class CleanupActions
    {
        /// <summary>Validates optional data removal before the MSI transaction changes any app files.</summary>
        [CustomAction]
        public static ActionResult PrepareRemoval(Session session)
        {
            try
            {
                UserDataCleanup.EnsureOperitStopped();
                if (session["OPERIT_DELETE_USER_DATA"] == "1")
                {
                    var cleanup = new UserDataCleanup(session["INSTALLFOLDER"]);
                    var json = new JavaScriptSerializer();
                    var plan = session["OPERIT_CLEANUP_INTERACTIVE"] == "1"
                        ? json.Deserialize<CleanupPlan>(session["OPERIT_CLEANUP_PLAN"]) : cleanup.CreatePlan();
                    cleanup.ValidatePlan(plan);
                    session["OPERIT_CLEANUP_PLAN"] = json.Serialize(plan);
                }
                return ActionResult.Success;
            }
            catch (Exception error) { return ReportFailure(session, error); }
        }

        /// <summary>Clears owned shell entries and explicitly selected data only after MSI removal commits.</summary>
        [CustomAction]
        public static ActionResult FinishRemoval(Session session)
        {
            try
            {
                UserDataCleanup.RemoveShellEntries(session["INSTALLFOLDER"], session["ProductCode"]);
                if (session["OPERIT_DELETE_USER_DATA"] == "1")
                    new UserDataCleanup(session["INSTALLFOLDER"]).DeleteData(
                        new JavaScriptSerializer().Deserialize<CleanupPlan>(session["OPERIT_CLEANUP_PLAN"]));
                return ActionResult.Success;
            }
            catch (Exception error)
            {
                session["OPERIT_CLEANUP_FAILED"] = "1";
                return ReportFailure(session, new InvalidOperationException(
                    "应用已移除，但清理未全部完成。已删除的数据无法恢复。请使用独立清理工具重试。\n" + error.Message));
            }
        }

        /// <summary>Publishes shortcuts after a successful installation or repair.</summary>
        [CustomAction]
        public static ActionResult RegisterShellEntries(Session session)
        {
            try
            {
                UserDataCleanup.RegisterShellEntries(session["INSTALLFOLDER"], session["ProductCode"]);
                return ActionResult.Success;
            }
            catch (Exception error) { return ReportFailure(session, error); }
        }

        /// <summary>Reports actionable errors to full UI and unattended MSI logs.</summary>
        private static ActionResult ReportFailure(Session session, Exception error)
        {
            using (var record = new Record(1))
            {
                record[0] = "[1]";
                record[1] = error.Message;
                session.Message(InstallMessage.Error, record);
            }
            return ActionResult.Failure;
        }
    }
}
