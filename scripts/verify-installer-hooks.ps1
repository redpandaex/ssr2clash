[CmdletBinding()]
param([string]$MakeNsis = "$env:LOCALAPPDATA/tauri/NSIS/Bin/makensis.exe")
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not $IsWindows) { throw 'Windows is required' }
$repoRoot = Split-Path -Parent $PSScriptRoot
$identity = [guid]::NewGuid().ToString('N')
$testRoot = Join-Path $repoRoot "src-tauri/target/installer-smoke/$identity"
$registrySubkey = "Software\SSR2Clash Installer Tests\$identity"
$registryRoot = "HKCU:\$registrySubkey"
$hooksPath = Join-Path $repoRoot 'src-tauri/installer/hooks.nsh'
New-Item -ItemType Directory -Path $testRoot -Force | Out-Null
New-Item -Path "$registryRoot\Run", "$registryRoot\Approved", "$registryRoot\Installed" -Force | Out-Null
$harnessSource = @'
Unicode true
RequestExecutionLevel user
SilentInstall silent
!include LogicLib.nsh
!include FileFunc.nsh
!define PRODUCTNAME "SSR2Clash Hook Test"
!define MAINBINARYNAME "ssr2clash"
!define MANUPRODUCTKEY "__REGISTRY__\Installed"
!define SSR2CLASH_RUN_KEY "__REGISTRY__\Run"
!define SSR2CLASH_APPROVED_KEY "__REGISTRY__\Approved"
!include "__HOOKS__"
Name "SSR2Clash isolated hook verification"
OutFile "__OUTPUT__"
Var UpdateMode
Var TestCase
Section
  SetShellVarContext current
  ${GetOptions} $CMDLINE "/CASE=" $TestCase
  ${If} $TestCase == "probe-file"
    StrCpy $INSTDIR "$INSTDIR\existing-file"
  ${EndIf}
  StrCpy $UpdateMode 0
  ${If} $TestCase == "update"
    StrCpy $UpdateMode 1
  ${EndIf}
  ${If} $TestCase == "remove"
  ${OrIf} $TestCase == "update"
    !insertmacro SSR2CLASH_REMOVE_OWN_AUTOSTART
  ${Else}
    !insertmacro NSIS_HOOK_PREINSTALL
    ${If} $TestCase == "migrate-raced"
      WriteRegStr HKCU "${SSR2CLASH_RUN_KEY}" "${PRODUCTNAME}" "changed-during-install"
    ${EndIf}
    !insertmacro NSIS_HOOK_POSTINSTALL
  ${EndIf}
  FileOpen $R3 "$INSTDIR\result.txt" w
  FileWrite $R3 "complete"
  FileClose $R3
  SetErrorLevel 0
SectionEnd
'@
$harnessExe = Join-Path $testRoot 'hooks-test.exe'
$harnessSource = $harnessSource.Replace('__REGISTRY__', $registrySubkey).Replace('__HOOKS__', $hooksPath).Replace('__OUTPUT__', $harnessExe)
$harnessFile = Join-Path $testRoot 'hooks-test.nsi'
[IO.File]::WriteAllText($harnessFile, $harnessSource, [Text.UTF8Encoding]::new($true))
function Set-TestRegistration([string]$Command) {
    New-ItemProperty -LiteralPath "$registryRoot\Run" -Name 'SSR2Clash Hook Test' -Value $Command -PropertyType String -Force | Out-Null
    New-ItemProperty -LiteralPath "$registryRoot\Approved" -Name 'SSR2Clash Hook Test' -Value ([byte[]]@(2,0,0,0,0,0,0,0,0,0,0,0)) -PropertyType Binary -Force | Out-Null
}
function Get-TestRegistration { (Get-Item -LiteralPath "$registryRoot\Run").GetValue('SSR2Clash Hook Test', $null) }
function Invoke-Case([string]$Case, [string]$Directory, [bool]$Success = $true) {
    $process = Start-Process -FilePath $harnessExe -ArgumentList @('/S', "/CASE=$Case", "/D=$Directory") -PassThru -Wait -WindowStyle Hidden
    if ($Success -and $process.ExitCode -ne 0) { throw "$Case failed with $($process.ExitCode)" }
    if (-not $Success -and $process.ExitCode -eq 0) { throw "$Case unexpectedly allowed the invalid folder" }
}
try {
    & $MakeNsis /V2 $harnessFile
    if ($LASTEXITCODE -ne 0) { throw 'NSIS hook harness failed to compile' }
    $installed = Join-Path $testRoot '安装 目录'
    $portable = Join-Path $testRoot '便携 目录'
    New-Item -ItemType Directory -Path $installed, $portable -Force | Out-Null
    $ownCommand = '"' + $installed + '\ssr2clash.exe" --autostart'
    $foreignCommand = '"' + $portable + '\ssr2clash.exe" --autostart'
    Set-TestRegistration $ownCommand
    Invoke-Case 'remove' $installed
    if ($null -ne (Get-TestRegistration)) { throw 'Own registration was not removed' }
    if ($null -ne (Get-Item -LiteralPath "$registryRoot\Approved").GetValue('SSR2Clash Hook Test', $null)) { throw 'Own approved-state entry was not removed' }
    Set-TestRegistration $foreignCommand
    Invoke-Case 'remove' $installed
    if ((Get-TestRegistration) -cne $foreignCommand) { throw 'Another copy lost its registration' }
    Set-TestRegistration $ownCommand
    Invoke-Case 'update' $installed
    if ((Get-TestRegistration) -cne $ownCommand) { throw 'Update removed the registration' }
    Set-Item -LiteralPath "$registryRoot\Installed" -Value $installed
    $newDirectory = Join-Path $testRoot '升级 新目录'
    Invoke-Case 'migrate' $newDirectory
    if ((Get-TestRegistration) -cne ('"' + $newDirectory + '\ssr2clash.exe" --autostart')) { throw 'Upgrade did not migrate its own startup path' }
    Set-TestRegistration $foreignCommand
    Invoke-Case 'migrate' $newDirectory
    if ((Get-TestRegistration) -cne $foreignCommand) { throw 'Upgrade stole another copy startup path' }
    Set-TestRegistration $ownCommand
    Invoke-Case 'migrate-raced' $newDirectory
    if ((Get-TestRegistration) -cne 'changed-during-install') { throw 'Upgrade overwrote a concurrent registration change' }
    $portableMarker = Join-Path $portable 'portable.flag'
    [IO.File]::WriteAllText($portableMarker, 'preserve-marker')
    Invoke-Case 'probe' $portable $false
    if ([IO.File]::ReadAllText($portableMarker) -cne 'preserve-marker') { throw 'Portable marker was changed' }
    $notDirectory = Join-Path $testRoot 'existing-file'
    [IO.File]::WriteAllText($notDirectory, 'preserve-file')
    Invoke-Case 'probe-file' $testRoot $false
    if ([IO.File]::ReadAllText($notDirectory) -cne 'preserve-file') { throw 'Invalid path file was changed' }
    Write-Output 'PASS eight isolated native NSIS cases: ownership, update preservation, path migration, foreign/concurrent registration, portable marker and non-directory rejection'
    Write-Output "Harness artifacts: $testRoot"
} finally {
    if (Test-Path -LiteralPath $registryRoot) {
        $resolvedRegistry = (Get-Item -LiteralPath $registryRoot).Name
        if ($resolvedRegistry -cne "HKEY_CURRENT_USER\$registrySubkey") { throw 'Unsafe test registry cleanup path' }
        Remove-Item -LiteralPath $registryRoot -Recurse -Force
    }
}
