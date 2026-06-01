$ErrorActionPreference = "Stop"

$runtimeDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$downloadsDir = Join-Path $runtimeDir "downloads"
$sourcesDir = Join-Path $runtimeDir "sources"

New-Item -ItemType Directory -Force -Path $downloadsDir | Out-Null
New-Item -ItemType Directory -Force -Path $sourcesDir | Out-Null

$items = @(
    @{
        Name = "busybox-1.38.0"
        Url = "https://busybox.net/downloads/busybox-1.38.0.tar.bz2"
        Archive = "busybox-1.38.0.tar.bz2"
        Sha256 = "34F9EA6FF8636F2C9241153B9114EEFA9E65674A45318AE1EF95BB5F31C53BB2"
        Extracted = "busybox-1.38.0"
    },
    @{
        Name = "proot-v5.4.0"
        Url = "https://github.com/proot-me/proot/archive/refs/tags/v5.4.0.tar.gz"
        Archive = "proot-v5.4.0.tar.gz"
        Sha256 = "29248AAC2A7CE10C3BD5EE5602742EC33B2532310FF9CF73B79F3C133E5A5F68"
        Extracted = "proot-5.4.0"
    },
    @{
        Name = "talloc-2.4.3"
        Url = "https://www.samba.org/ftp/talloc/talloc-2.4.3.tar.gz"
        Archive = "talloc-2.4.3.tar.gz"
        Sha256 = "DC46C40B9F46BB34DD97FE41F548B0E8B247B77A918576733C528E83ABD854DD"
        Extracted = "talloc-2.4.3"
    },
    @{
        Name = "bash-5.2.37"
        Url = "https://ftp.gnu.org/gnu/bash/bash-5.2.37.tar.gz"
        Archive = "bash-5.2.37.tar.gz"
        Sha256 = "9599B22ECD1D5787AD7D3B7BF0C59F312B3396D1E281175DD1F8A4014DA621FF"
        Extracted = "bash-5.2.37"
    }
)

foreach ($item in $items) {
    $archivePath = Join-Path $downloadsDir $item.Archive
    Write-Host "Downloading $($item.Name)"
    Invoke-WebRequest -Uri $item.Url -OutFile $archivePath

    $actualSha256 = (Get-FileHash -Algorithm SHA256 -Path $archivePath).Hash.ToUpperInvariant()
    if ($actualSha256 -ne $item.Sha256) {
        throw "$($item.Archive) SHA256 mismatch: $actualSha256"
    }

    $extractPath = Join-Path $sourcesDir $item.Extracted
    if (Test-Path $extractPath) {
        Remove-Item -LiteralPath $extractPath -Recurse -Force
    }

    tar -xf $archivePath -C $sourcesDir
}
