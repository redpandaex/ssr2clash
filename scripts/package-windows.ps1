[CmdletBinding()]
param([switch]$SkipBuild, [string]$OutputDirectory = 'src-tauri/target/distribution')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows -or $PSVersionTable.PSVersion.Major -lt 7) { throw 'Build packages with PowerShell 7 on Windows' }
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $package = Get-Content -LiteralPath package.json -Raw | ConvertFrom-Json
    $config = Get-Content -LiteralPath src-tauri/tauri.conf.json -Raw | ConvertFrom-Json
    $version = $package.version
    $cargoText = Get-Content -LiteralPath src-tauri/Cargo.toml -Raw
    $cargoPackage = [regex]::Match($cargoText, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)').Groups[1].Value
    $cargoVersion = [regex]::Match($cargoPackage, '(?m)^version\s*=\s*"([^"]+)"\s*$').Groups[1].Value
    if ($config.version -ne $version -or $cargoVersion -ne $version) { throw 'Application versions must match in package.json, Cargo.toml and tauri.conf.json' }
    if (-not $SkipBuild) {
        & "$PSScriptRoot/collect-notices.ps1"
        & corepack pnpm tauri build --target x86_64-pc-windows-msvc --bundles nsis
        if ($LASTEXITCODE -ne 0) { throw 'Tauri Windows build failed' }
    }
    $releaseRoot = Join-Path $repoRoot 'src-tauri/target/x86_64-pc-windows-msvc/release'
    $exe = Join-Path $releaseRoot 'ssr2clash.exe'
    $installer = Join-Path $releaseRoot "bundle/nsis/SSR2Clash_${version}_x64-setup.exe"
    foreach ($required in @($exe, $installer)) { if (-not (Test-Path -LiteralPath $required -PathType Leaf)) { throw "Missing build output: $required" } }
    $outputRoot = if ([IO.Path]::IsPathRooted($OutputDirectory)) { [IO.Path]::GetFullPath($OutputDirectory) } else { [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory)) }
    if (-not $outputRoot.StartsWith($repoRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Package output must stay inside this checkout' }
    New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
    $stageRoot = Join-Path $outputRoot ('stage-' + [guid]::NewGuid().ToString('N'))
    $portable = Join-Path $outputRoot "ssr2clash_${version}_windows_x64_portable.zip"
    $setup = Join-Path $outputRoot "ssr2clash_${version}_windows_x64_setup.exe"
    try {
        New-Item -ItemType Directory -Path (Join-Path $stageRoot 'licenses') -Force | Out-Null
        Copy-Item -LiteralPath $exe -Destination (Join-Path $stageRoot 'ssr2clash.exe')
        [IO.File]::WriteAllBytes((Join-Path $stageRoot 'portable.flag'), [byte[]]@())
        Copy-Item -LiteralPath src-tauri/resources/portable-README.txt -Destination (Join-Path $stageRoot 'README.txt')
        Copy-Item -LiteralPath src-tauri/resources/THIRD-PARTY-NOTICES.txt -Destination $stageRoot
        Copy-Item -LiteralPath src-tauri/resources/presets/default/LICENSE -Destination (Join-Path $stageRoot 'licenses/subconverter-GPL-3.0.txt')
        foreach ($license in (Get-ChildItem -LiteralPath src-tauri/resources/licenses -File)) {
            Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $stageRoot 'licenses')
        }
        if (Test-Path -LiteralPath $portable) { Remove-Item -LiteralPath $portable }
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        [IO.Compression.ZipFile]::CreateFromDirectory($stageRoot, $portable, [IO.Compression.CompressionLevel]::Optimal, $false)
        Copy-Item -LiteralPath $installer -Destination $setup -Force
    } finally {
        if (Test-Path -LiteralPath $stageRoot) {
            $resolvedStage = [IO.Path]::GetFullPath($stageRoot)
            if (-not $resolvedStage.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe package staging cleanup path' }
            Remove-Item -LiteralPath $resolvedStage -Recurse -Force
        }
    }
    & "$PSScriptRoot/verify-windows-packages.ps1" -OutputDirectory $outputRoot
    $commit = (& git rev-parse HEAD).Trim()
    $dirty = [bool](& git status --porcelain)
    $buildInfo = [ordered]@{ version = $version; commit = $commit; workingTreeDirty = $dirty; target = 'x86_64-pc-windows-msvc'; generatedAt = [DateTime]::UtcNow.ToString('o') }
    $buildInfo | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $outputRoot 'build-info.json') -Encoding utf8NoBOM
    $hashes = foreach ($artifact in @($portable, $setup)) { "$( (Get-FileHash -LiteralPath $artifact -Algorithm SHA256).Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($artifact))" }
    $hashes | Set-Content -LiteralPath (Join-Path $outputRoot 'SHA256SUMS.txt') -Encoding utf8NoBOM
    Write-Output "Windows packages ready: $outputRoot"
} finally { Pop-Location }
