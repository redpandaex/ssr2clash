[CmdletBinding()]
param([string]$OutputDirectory = 'src-tauri/target/distribution')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = Split-Path -Parent $PSScriptRoot
$package = Get-Content -LiteralPath (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json
$version = $package.version
$outputRoot = if ([IO.Path]::IsPathRooted($OutputDirectory)) { [IO.Path]::GetFullPath($OutputDirectory) } else { [IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDirectory)) }
$portable = Join-Path $outputRoot "ssr2clash_${version}_windows_x64_portable.zip"
$setup = Join-Path $outputRoot "ssr2clash_${version}_windows_x64_setup.exe"
foreach ($artifact in @($portable, $setup)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) { throw "Missing artifact: $artifact" }
    if ((Get-Item -LiteralPath $artifact).Length -eq 0) { throw "Empty artifact: $artifact" }
}
$expectedFiles = @('ssr2clash.exe', 'portable.flag', 'README.txt', 'THIRD-PARTY-NOTICES.txt', 'licenses/subconverter-GPL-3.0.txt', 'licenses/tauri-MIT.txt', 'licenses/tauri-APACHE-2.0.txt', 'licenses/geist-OFL-1.1.txt', 'licenses/geist-mono-OFL-1.1.txt', 'licenses/frontend-LICENSES.txt', 'licenses/if-addrs-MIT.txt')
Add-Type -AssemblyName System.IO.Compression.FileSystem
$archive = [IO.Compression.ZipFile]::OpenRead($portable)
$temporaryRoot = Join-Path $outputRoot ("verify-" + [guid]::NewGuid().ToString('N'))
try {
    $actualFiles = @($archive.Entries | Where-Object { $_.Name } | ForEach-Object { $_.FullName.Replace('\', '/') })
    if (@(Compare-Object ($expectedFiles | Sort-Object) ($actualFiles | Sort-Object)).Count) { throw "Unexpected ZIP contents: $($actualFiles -join ', ')" }
    if ($actualFiles | Where-Object { $_ -match '(^|/)(data|state\.json)(/|$)|\.(yaml|yml|ini)$' }) { throw 'Portable archive contains user configuration' }
    if ($archive.GetEntry('portable.flag').Length -ne 0) { throw 'portable.flag must be empty' }
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    $exe = Join-Path $temporaryRoot 'ssr2clash.exe'
    [IO.Compression.ZipFileExtensions]::ExtractToFile($archive.GetEntry('ssr2clash.exe'), $exe, $true)
    foreach ($binary in @($exe, $setup)) {
        $metadata = [Diagnostics.FileVersionInfo]::GetVersionInfo($binary)
        if ($metadata.ProductName -ne 'SSR2Clash') { throw "Unexpected product name in $binary" }
        if ($metadata.ProductVersion -notmatch ('^' + [regex]::Escape($version) + '(\.0)?$')) { throw "Product version mismatch: $($metadata.ProductVersion) in $binary" }
        if ($metadata.FileVersion -notmatch ('^' + [regex]::Escape($version) + '(\.0)?$')) { throw "File version mismatch: $($metadata.FileVersion) in $binary" }
    }
} finally {
    $archive.Dispose()
    if (Test-Path -LiteralPath $temporaryRoot) {
        $resolvedTemporary = [IO.Path]::GetFullPath($temporaryRoot)
        if (-not $resolvedTemporary.StartsWith($outputRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe verification cleanup path' }
        Remove-Item -LiteralPath $resolvedTemporary -Recurse -Force
    }
}
Write-Output "PASS Windows packages $version; exact portable contents; no preconfigured user data; executable and installer product versions match"
Get-Item -LiteralPath $portable, $setup | Select-Object Name, Length
