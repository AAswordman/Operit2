"""Exercise Windows self-update policy and helper sequencing without installing software."""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


REPO = Path(__file__).resolve().parents[2]
SOURCE = REPO / "hosts/windows/src/tools/system/application_update.ps1"

WORKER_FIXTURE = r'''
$script:Updated = $false
$script:ExitCode = 0
$script:OtherInstance = $false
$script:InvalidPackage = $false
$script:Executable = Join-Path $env:OPERIT_UPDATE_TEST_ROOT "installed app\operit2.exe"
$script:Package = Join-Path $env:OPERIT_UPDATE_TEST_ROOT "operit2-app-windows-x86_64-2.0.0-preview.32.msi"
$script:Ready = Join-Path $env:OPERIT_UPDATE_TEST_ROOT 'helper.ready'
$script:Log = Join-Path $env:OPERIT_UPDATE_TEST_ROOT 'helper.log'
$script:Trace = Join-Path $env:OPERIT_UPDATE_TEST_ROOT 'trace.jsonl'

# Models installed product identity before and after the installer transaction.
function Get-OperitInstallation($Executable) {
    $version = '2.0.231'
    $code = '{00000000-0000-0000-0000-000000000031}'
    if ($script:Updated) { $version = '2.0.232'; $code = '{00000000-0000-0000-0000-000000000032}' }
    return [pscustomobject]@{ msi = $true; folder = [IO.Path]::GetDirectoryName($Executable); version = $version; productCode = $code }
}

# Supplies a package database identity without opening or installing a real MSI.
function Get-OperitMsiMetadata($Package) {
    $name = 'Operit2'
    if ($script:InvalidPackage) { $name = 'Unrelated product' }
    return [pscustomobject]@{
        productName = $name; manufacturer = 'Operit'; upgradeCode = $OperitUpgradeCode
        architecture = 'x64'; allUsers = ''; release = '2.0.0-preview.32'; version = '2.0.232'
        productCode = '{00000000-0000-0000-0000-000000000032}'
    }
}

# Records the exact parent wait and prevents installation before readiness.
$script:Parent = [pscustomobject]@{ MainModule = @{ FileName = $script:Executable }; Handle = 1 }
$script:Parent | Add-Member ScriptMethod WaitForExit {
    if (-not (Test-Path -LiteralPath $script:Ready)) { throw 'Parent wait began before readiness.' }
    Add-Content -LiteralPath $script:Trace -Value '{"event":"wait"}' -Encoding UTF8
}
$script:Parent | Add-Member ScriptMethod Dispose { }

# Models the owning process and any second instance without stopping real processes.
function Get-Process($Id, $Name, $ErrorAction) {
    if ($PSBoundParameters.ContainsKey('Id')) { return $script:Parent }
    if ($script:OtherInstance) { return [pscustomobject]@{ Path = $script:Executable } }
}

# Captures MSI arguments and relaunches without starting any executable.
function Start-Process($FilePath, $ArgumentList, $WindowStyle, [switch]$Wait, [switch]$PassThru, $WorkingDirectory) {
    if ([IO.Path]::GetFileName($FilePath) -eq 'msiexec.exe') {
        if (-not (Test-Path -LiteralPath $script:Trace)) { throw 'Installer ran before parent exit.' }
        Add-Content -LiteralPath $script:Trace -Value (@{ event = 'msi'; arguments = $ArgumentList; window = $WindowStyle; wait = $Wait.IsPresent } | ConvertTo-Json -Compress) -Encoding UTF8
        if ($script:ExitCode -eq 0) { $script:Updated = $true }
        return [pscustomobject]@{ ExitCode = $script:ExitCode }
    }
    Add-Content -LiteralPath $script:Trace -Value (@{ event = 'launch'; executable = $FilePath; folder = $WorkingDirectory } | ConvertTo-Json -Compress) -Encoding UTF8
}

# Captures exceptional feedback without opening a native dialog during tests.
function Show-OperitUpdateMessage($Message, $Title) {
    [IO.File]::WriteAllText((Join-Path $env:OPERIT_UPDATE_TEST_ROOT 'message.txt'), $Message)
}
'''


@unittest.skipUnless(sys.platform == "win32", "Windows Installer helpers require Windows")
class WindowsApplicationUpdateTests(unittest.TestCase):
    # Keep every helper artifact in an isolated directory, including Unicode and spaces.
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="operit update 中文 '")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    # Run the production PowerShell functions with isolated providers and capture failures.
    def run_script(self, body, expected_code=0):
        harness = self.root / "test.ps1"
        harness.write_text(
            "$ErrorActionPreference = 'Stop'\n. ([scriptblock]::Create([IO.File]::ReadAllText($env:OPERIT_UPDATE_SOURCE)))\n" + body,
            encoding="utf-8-sig",
        )
        environment = dict(os.environ, OPERIT_UPDATE_SOURCE=str(SOURCE), OPERIT_UPDATE_TEST_ROOT=str(self.root))
        process = subprocess.run(
            ["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", str(harness)],
            env=environment, capture_output=True, encoding="utf-8", errors="replace", timeout=30,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        self.assertEqual(process.returncode, expected_code, process.stdout + process.stderr)
        return process

    # Check both installation formats against the current copy, not another installed copy.
    def test_installed_directory_ownership_and_stale_registration(self):
        self.run_script(r'''
function Get-OperitRegisteredFolder { return $null }
function Get-OperitInstalledMsiProducts { throw 'Unregistered portable copies must not query MSI products.' }
if ((Get-OperitInstallation 'D:\Portable\operit2.exe').msi) { throw 'Unregistered portable copy was classified as MSI.' }
function Get-OperitRegisteredFolder { return 'C:\Users\Example\Operit2' }
function Get-OperitInstalledMsiProducts { throw 'Portable copies must not query unrelated MSI products.' }
if ((Get-OperitInstallation 'D:\Portable\operit2.exe').msi) { throw 'Portable copy was classified as MSI.' }
function Get-OperitInstalledMsiProducts { return [pscustomobject]@{ productCode = 'fixture'; version = '2.0.231' } }
if (-not (Get-OperitInstallation '\\?\C:\Users\Example\Operit2\operit2.exe').msi) { throw 'Extended paths lost MSI ownership.' }
function Get-OperitInstalledMsiProducts { }
$rejected = $false
try { Get-OperitInstallation 'C:\Users\Example\Operit2\operit2.exe' } catch { $rejected = $true }
if (-not $rejected) { throw 'Stale registration was accepted.' }
''')

    # Reject unrelated, downgraded, machine-wide, and misnamed packages before shutdown.
    def test_package_identity_validation(self):
        self.run_script(WORKER_FIXTURE + r'''
$installed = Get-OperitInstallation $script:Executable
$valid = Get-OperitMsiMetadata $script:Package
Assert-OperitMsiUpdate $installed $valid $script:Package
foreach ($change in @(
    @{ upgradeCode = '{00000000-0000-0000-0000-000000000000}' },
    @{ productName = 'Other' }, @{ manufacturer = 'Other' }, @{ architecture = 'Arm64' },
    @{ allUsers = '1' }, @{ release = '2.0.0-preview.33' }, @{ version = '2.0.230' },
    @{ productCode = $installed.productCode }, @{ productCode = 'not-a-product-code' }
)) {
    $metadata = Get-OperitMsiMetadata $script:Package
    foreach ($name in $change.Keys) { $metadata.$name = $change[$name] }
    $rejected = $false
    try { Assert-OperitMsiUpdate $installed $metadata $script:Package } catch { $rejected = $true }
    if (-not $rejected) { throw ('Invalid MSI metadata accepted: ' + ($change | ConvertTo-Json -Compress)) }
}
''')

    # Inspect persistent worker output after the owning application would have exited.
    def run_worker(self, settings="", expected_code=0):
        self.run_script(WORKER_FIXTURE + settings + "\nInvoke-OperitMsiUpdate 42 $script:Package $script:Executable $script:Ready $script:Log\n", expected_code)
        result = json.loads((self.root / "helper.log.result.json").read_text(encoding="utf-8-sig"))
        trace_path = self.root / "trace.jsonl"
        trace = [json.loads(line) for line in trace_path.read_text(encoding="utf-8-sig").splitlines()] if trace_path.exists() else []
        return result, trace

    # Require parent exit, silent installation, preserved destination, and exactly one relaunch.
    def test_success_waits_installs_silently_and_relaunches(self):
        result, trace = self.run_worker()
        self.assertEqual(result["state"], "installed")
        self.assertEqual([event["event"] for event in trace], ["wait", "msi", "launch"])
        self.assertIn('/qn /norestart', trace[1]["arguments"])
        self.assertIn('INSTALLFOLDER="' + str(self.root / "installed app") + '"', trace[1]["arguments"])
        self.assertEqual(trace[1]["window"], "Hidden")
        self.assertTrue(trace[1]["wait"])
        self.assertEqual(trace[2]["executable"], str(self.root / "installed app/operit2.exe"))

    # A preparation failure must not publish readiness or request application exit.
    def test_invalid_package_never_becomes_ready(self):
        result, trace = self.run_worker("$script:InvalidPackage = $true", expected_code=1)
        self.assertEqual(result["state"], "failed")
        self.assertFalse((self.root / "helper.ready").exists())
        self.assertFalse((self.root / "message.txt").exists())
        self.assertEqual(trace, [])

    # MSI failure must preserve evidence and must not launch an unverified application.
    def test_installer_failure_is_recorded_and_reported(self):
        result, trace = self.run_worker("$script:ExitCode = 1603", expected_code=1)
        self.assertEqual(result["state"], "failed")
        self.assertIn("1603", result["details"])
        self.assertTrue((self.root / "message.txt").exists())
        self.assertEqual([event["event"] for event in trace], ["wait", "msi"])

    # Keep a Windows reboot request distinct from an ordinary application restart.
    def test_system_restart_is_recorded_without_claiming_app_restart(self):
        result, trace = self.run_worker("$script:ExitCode = 3010")
        self.assertEqual(result["state"], "system_restart_required")
        self.assertTrue((self.root / "message.txt").exists())
        self.assertEqual([event["event"] for event in trace], ["wait", "msi"])

    # Another process using this installation must prevent MSI from replacing its files.
    def test_second_instance_blocks_installation(self):
        result, trace = self.run_worker("$script:OtherInstance = $true", expected_code=1)
        self.assertEqual(result["state"], "failed")
        self.assertEqual([event["event"] for event in trace], ["wait"])


if __name__ == "__main__":
    unittest.main()
