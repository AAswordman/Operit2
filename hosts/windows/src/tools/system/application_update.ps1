$ErrorActionPreference = 'Stop'
$OperitUpgradeCode = '{A60B4E40-8A60-4BCD-AB33-DA7D939E436D}'

# Normalizes ordinary and extended Windows paths for installation ownership checks.
function ConvertTo-OperitPath([string]$Path) {
    if ($Path.StartsWith('\\?\UNC\', [StringComparison]::OrdinalIgnoreCase)) {
        $Path = '\\' + $Path.Substring(8)
    } elseif ($Path.StartsWith('\\?\', [StringComparison]::Ordinal)) {
        $Path = $Path.Substring(4)
    }
    return [IO.Path]::GetFullPath($Path).TrimEnd('\')
}

# Reads the current user's MSI-owned directory without modifying registration.
function Get-OperitRegisteredFolder {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Operit\Operit2\Installer')
    if ($null -eq $key) { return $null }
    try { $registered = [string]$key.GetValue('InstallLocation') }
    finally { $key.Dispose() }
    if ([string]::IsNullOrWhiteSpace($registered)) { throw 'Operit MSI registration is missing InstallLocation.' }
    return $registered
}

# Enumerates the current user's related MSI products through Windows Installer.
function Get-OperitInstalledMsiProducts {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $related = $null
    try {
        $related = $installer.RelatedProducts($OperitUpgradeCode)
        foreach ($product in $related) {
            if ([string]$installer.ProductInfo($product, 'AssignmentType') -eq '0') {
                [pscustomobject]@{
                    productCode = [string]$product
                    version = [string]$installer.ProductInfo($product, 'VersionString')
                }
            }
        }
    } finally {
        if ($null -ne $related) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($related) }
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
    }
}

# Resolves MSI ownership only when its registered directory is this running copy.
function Get-OperitInstallation([string]$Executable) {
    $folder = ConvertTo-OperitPath ([IO.Path]::GetDirectoryName($Executable))
    $registered = Get-OperitRegisteredFolder
    if ($null -eq $registered -or -not [string]::Equals($folder, (ConvertTo-OperitPath $registered), [StringComparison]::OrdinalIgnoreCase)) {
        return [pscustomobject]@{ msi = $false; folder = $folder }
    }
    $products = @(Get-OperitInstalledMsiProducts)
    if ($products.Count -ne 1) { throw 'The current user must have exactly one registered Operit MSI product.' }
    return [pscustomobject]@{
        msi = $true
        folder = $folder
        productCode = $products[0].productCode
        version = $products[0].version
    }
}

# Reads package identity without executing MSI actions or holding the database open.
function Get-OperitMsiMetadata([string]$Package) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $null
    $view = $null
    $summary = $null
    try {
        $database = $installer.OpenDatabase($Package, 0)
        $view = $database.OpenView('SELECT `Property`, `Value` FROM `Property`')
        [void]$view.Execute()
        $properties = @{}
        while ($null -ne ($record = $view.Fetch())) {
            try { $properties[[string]$record.StringData(1)] = [string]$record.StringData(2) }
            finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($record) }
        }
        $summary = $database.SummaryInformation(0)
        return [pscustomobject]@{
            upgradeCode = $properties.UpgradeCode
            productCode = $properties.ProductCode
            productName = $properties.ProductName
            manufacturer = $properties.Manufacturer
            version = $properties.ProductVersion
            release = $properties.OPERIT_RELEASE_VERSION
            allUsers = $properties.ALLUSERS
            architecture = ([string]$summary.Property(7)).Split(';')[0]
        }
    } finally {
        if ($null -ne $summary) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($summary) }
        if ($null -ne $view) { [void]$view.Close(); [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view) }
        if ($null -ne $database) { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database) }
        [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
    }
}

# Rejects unrelated, machine-wide, misnamed, and non-upgrade packages before shutdown.
function Assert-OperitMsiUpdate($Installation, $Metadata, [string]$Package) {
    if (-not $Installation.msi) { throw 'The running application is not owned by an Operit MSI installation.' }
    if ($Metadata.upgradeCode -ne $OperitUpgradeCode -or $Metadata.productName -cne 'Operit2' -or $Metadata.manufacturer -cne 'Operit') {
        throw 'The update package does not belong to the Operit MSI product family.'
    }
    if ($Metadata.architecture -cne 'x64' -or -not [string]::IsNullOrEmpty($Metadata.allUsers)) {
        throw 'The update must be a per-user x64 MSI package.'
    }
    if ([string]::IsNullOrWhiteSpace($Metadata.release) -or
        [IO.Path]::GetFileName($Package) -cne ('operit2-app-windows-x86_64-' + $Metadata.release + '.msi')) {
        throw 'The MSI release identity does not match its update asset name.'
    }
    [void][guid]::Parse($Metadata.productCode)
    if ([version]$Metadata.version -le [version]$Installation.version -or $Metadata.productCode -eq $Installation.productCode) {
        throw 'The MSI update must contain a newer product version and a new product code.'
    }
}

# Persists the installer's outcome independently of the application process.
function Write-OperitUpdateResult([string]$Log, [string]$State, [string]$Version, [string]$Details) {
    $result = [ordered]@{ state = $State; version = $Version; details = $Details; log = $Log + '.msi.log' }
    $path = $Log + '.result.json'
    [IO.File]::WriteAllText($path + '.tmp', ($result | ConvertTo-Json -Compress))
    Move-Item -LiteralPath ($path + '.tmp') -Destination $path -Force
}

# Reports an exceptional installer outcome after the application has exited.
function Show-OperitUpdateMessage([string]$Message, [string]$Title) {
    Add-Type -AssemblyName System.Windows.Forms
    [void][System.Windows.Forms.MessageBox]::Show($Message, $Title)
}

# Runs one unattended upgrade after the exact owning process has fully exited.
function Invoke-OperitMsiUpdate([int]$ParentPid, [string]$Package, [string]$Executable, [string]$Ready, [string]$Log) {
    $readyPublished = $false
    $metadata = $null
    try {
        $installation = Get-OperitInstallation $Executable
        $metadata = Get-OperitMsiMetadata $Package
        Assert-OperitMsiUpdate $installation $metadata $Package
        $parent = Get-Process -Id $ParentPid -ErrorAction Stop
        if (-not [string]::Equals((ConvertTo-OperitPath $parent.MainModule.FileName), (ConvertTo-OperitPath $Executable), [StringComparison]::OrdinalIgnoreCase)) {
            throw 'The updater parent does not match the installed executable.'
        }
        $null = $parent.Handle
        Write-OperitUpdateResult $Log 'prepared' $metadata.release ''
        [IO.File]::WriteAllText($Ready, 'ready')
        $readyPublished = $true
        $parent.WaitForExit()
        $parent.Dispose()
        $running = @(Get-Process -Name ([IO.Path]::GetFileNameWithoutExtension($Executable)) -ErrorAction SilentlyContinue |
            Where-Object { [string]::Equals((ConvertTo-OperitPath $_.Path), (ConvertTo-OperitPath $Executable), [StringComparison]::OrdinalIgnoreCase) })
        if ($running.Count -gt 0) { throw 'Another instance is using this Operit installation. Close it before updating.' }
        $current = Get-OperitInstallation $Executable
        if ($current.productCode -ne $installation.productCode -or $current.version -ne $installation.version) {
            throw 'The installed MSI product changed while this update was waiting.'
        }
        $pending = Get-OperitMsiMetadata $Package
        Assert-OperitMsiUpdate $current $pending $Package
        if ($pending.productCode -ne $metadata.productCode -or $pending.version -ne $metadata.version) {
            throw 'The prepared MSI package identity changed while waiting for application exit.'
        }
        $arguments = '/i "' + $Package + '" /qn /norestart /l*v "' + $Log + '.msi.log" INSTALLFOLDER="' +
            $installation.folder + '" MSIRESTARTMANAGERCONTROL=Disable MSIDISABLERMRESTART=1'
        $process = Start-Process -FilePath (Join-Path ([Environment]::SystemDirectory) 'msiexec.exe') -ArgumentList $arguments -WindowStyle Hidden -Wait -PassThru
        if ($process.ExitCode -eq 3010) {
            Write-OperitUpdateResult $Log 'system_restart_required' $metadata.release 'Windows restart is required to finish installation.'
            Show-OperitUpdateMessage 'Operit 更新需要重启 Windows 才能完成。请保存其他工作后重启电脑。' 'Operit 更新'
            return
        }
        if ($process.ExitCode -ne 0) { throw "Windows Installer failed with exit code $($process.ExitCode)." }
        $updated = Get-OperitInstallation $Executable
        if ($updated.productCode -ne $metadata.productCode -or [version]$updated.version -ne [version]$metadata.version) {
            throw 'Windows Installer did not register the expected updated product.'
        }
        Write-OperitUpdateResult $Log 'installed' $metadata.release ''
        Start-Process -FilePath $Executable -WorkingDirectory $installation.folder
    } catch {
        $details = $_ | Out-String
        Write-Output $details
        $version = ''
        if ($null -ne $metadata) { $version = $metadata.release }
        Write-OperitUpdateResult $Log 'failed' $version $details
        if ($readyPublished) {
            Show-OperitUpdateMessage ('Operit 更新未完成。' + [Environment]::NewLine + $details + [Environment]::NewLine + '日志：' + $Log) 'Operit 更新失败'
        }
        exit 1
    }
}
