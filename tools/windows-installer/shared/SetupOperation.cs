using System;
using System.Collections.Generic;
using System.Linq;

namespace Operit.Installer
{
    public enum SetupOperation { Install, Update, Repair, Remove }

    public static class SetupOperationSelection
    {
        /// <summary>Compares MSI versions across related products before choosing install or maintenance.</summary>
        public static SetupOperation Resolve(bool removing, Version packageVersion,
            IEnumerable<Version> installedVersions)
        {
            if (removing) return SetupOperation.Remove;
            var currentVersion = installedVersions.OrderByDescending(version => version).FirstOrDefault();
            if (currentVersion != null)
            {
                int comparison = packageVersion.CompareTo(currentVersion);
                if (comparison < 0)
                    throw new InvalidOperationException("已安装更新版本的 Operit2，请使用更新的安装包。");
                return comparison > 0 ? SetupOperation.Update : SetupOperation.Repair;
            }
            return SetupOperation.Install;
        }
    }
}
