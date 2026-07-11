param(
    [Parameter(Mandatory = $true)]
    [string]$Scenario,

    [string]$ReportsDir = "tools/link-perf/reports"
)

$ErrorActionPreference = "Stop"

# Resolves the scenario path to an absolute path.
function Resolve-ScenarioPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return (Resolve-Path -LiteralPath $Path).Path
}

# Creates a timestamped report path for a scenario run.
function New-ReportPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Directory,

        [Parameter(Mandatory = $true)]
        [string]$ScenarioPath
    )

    New-Item -ItemType Directory -Force -Path $Directory | Out-Null
    $scenarioName = [System.IO.Path]::GetFileNameWithoutExtension($ScenarioPath)
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    return Join-Path $Directory "$scenarioName-$timestamp.json"
}

# Reads a scenario JSON file into an object.
function Read-Scenario {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    return Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
}

$scenarioPath = Resolve-ScenarioPath -Path $Scenario
$scenarioData = Read-Scenario -Path $scenarioPath
$reportPath = New-ReportPath -Directory $ReportsDir -ScenarioPath $scenarioPath

$report = [ordered]@{
    scenario = $scenarioData.name
    kind = $scenarioData.kind
    scenarioPath = $scenarioPath
    createdAt = (Get-Date).ToString("o")
    command = $scenarioData.command
    notes = "Scenario metadata captured. Wire process execution in this script when the scenario command is finalized."
}

$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "link perf report: $reportPath"
