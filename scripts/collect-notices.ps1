$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $reportText = & corepack pnpm licenses list --prod --json
    if ($LASTEXITCODE -ne 0) { throw 'Unable to collect frontend licenses' }
    $report = ($reportText -join "`n") | ConvertFrom-Json -AsHashtable
    $text = [Text.StringBuilder]::new()
    [void]$text.AppendLine('Frontend dependency notices generated from the pinned pnpm lockfile.')
    foreach ($group in $report.Values) {
        foreach ($dependency in ($group | Sort-Object name)) {
            [void]$text.AppendLine("`n================ $($dependency.name) $($dependency.versions -join ', ') ================")
            [void]$text.AppendLine("License: $($dependency.license)")
            if ($dependency.ContainsKey('homepage') -and $dependency.homepage) { [void]$text.AppendLine($dependency.homepage) }
            $licenseFiles = @(Get-ChildItem -LiteralPath $dependency.paths[0] -File | Where-Object { $_.Name -match '^(LICEN[SC]E|COPYING|NOTICE)(\.|$|-)' })
            foreach ($file in $licenseFiles) {
                [void]$text.AppendLine("`n$($file.Name)")
                [void]$text.AppendLine((Get-Content -LiteralPath $file.FullName -Raw))
            }
        }
    }
    $destination = Join-Path $repoRoot 'src-tauri/resources/licenses/frontend-LICENSES.txt'
    $content = $text.ToString()
    if (-not (Test-Path -LiteralPath $destination) -or [IO.File]::ReadAllText($destination) -cne $content) {
        [IO.File]::WriteAllText($destination, $content, [Text.UTF8Encoding]::new($false))
    }
    Write-Output 'Collected pinned frontend dependency notices and available license texts'
} finally { Pop-Location }
