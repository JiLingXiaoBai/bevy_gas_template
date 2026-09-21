#requires -Version 7.2
<#
.SYNOPSIS
Generates, compiles, validates, and publishes the project's Rust configuration package.
.DESCRIPTION
Uses the pinned Luban toolchain and custom Rust module templates. New code and binary data
are staged and validated together before replacing either published output directory.
Run tools/luban/setup.ps1 before the first export. Excel sources are never rewritten.
.OUTPUTS
Validated Rust modules under config/generated and binary data plus manifest.json under assets/config.
#>
[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Filesystem and process operations used by the export pipeline.
function Get-ExportFullPath {
    param([Parameter(Mandatory)][string]$Path)
    return [System.IO.Path]::GetFullPath($Path).TrimEnd([char[]]@('/', '\'))
}

function Test-ExportPathWithin {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Root)
    $resolvedPath = Get-ExportFullPath $Path
    $resolvedRoot = Get-ExportFullPath $Root
    return $resolvedPath.Equals($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase) -or
        $resolvedPath.StartsWith($resolvedRoot + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)
}

function Assert-ExportPath {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string[]]$AllowedRoots
    )
    $resolvedPath = Get-ExportFullPath $Path
    if (-not (Test-ExportPathWithin $resolvedPath $RepositoryRoot)) {
        throw "Export path is outside the repository: $resolvedPath"
    }
    $allowed = $false
    foreach ($root in $AllowedRoots) {
        if (Test-ExportPathWithin $resolvedPath $root) { $allowed = $true; break }
    }
    if (-not $allowed) { throw "Export path is outside the allowed locations: $resolvedPath" }

    $current = $resolvedPath
    while ($true) {
        $attributes = $null
        try { $attributes = [System.IO.File]::GetAttributes($current) }
        catch [System.IO.FileNotFoundException] {}
        catch [System.IO.DirectoryNotFoundException] {}
        if ($null -ne $attributes -and ($attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
            throw "Export paths must not contain reparse points: $current"
        }
        if ($current.Equals((Get-ExportFullPath $RepositoryRoot), [System.StringComparison]::OrdinalIgnoreCase)) { break }
        $current = Split-Path -Parent $current
    }
}

function Assert-ExportPlainTree {
    param([Parameter(Mandatory)][string]$Path)
    $directory = Get-Item -LiteralPath $Path -Force
    if (-not $directory.PSIsContainer -or ($directory.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
        throw "Expected a regular export directory: $Path"
    }
    $pending = [System.Collections.Generic.Stack[string]]::new()
    $pending.Push($directory.FullName)
    while ($pending.Count -gt 0) {
        foreach ($item in Get-ChildItem -LiteralPath $pending.Pop() -Force) {
            if ($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) {
                throw "Export directories must not contain reparse points: $($item.FullName)"
            }
            if ($item.PSIsContainer) { $pending.Push($item.FullName) }
        }
    }
}

function New-ExportDirectory {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    Assert-ExportPath $Path $RepositoryRoot @($StageRoot)
    if (Test-Path -LiteralPath $Path) {
        if (-not (Test-Path -LiteralPath $Path -PathType Container)) { throw "Not a directory: $Path" }
    } else {
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
}

function Copy-ExportDirectory {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    Assert-ExportPath $Source $RepositoryRoot @($RepositoryRoot)
    Assert-ExportPlainTree $Source
    Assert-ExportPath $Destination $RepositoryRoot @($StageRoot)
    if (Test-Path -LiteralPath $Destination) { throw "Copy destination already exists: $Destination" }
    New-ExportDirectory (Split-Path -Parent $Destination) $RepositoryRoot $StageRoot
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse
}

function Copy-ExportFile {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    Assert-ExportPath $Source $RepositoryRoot @($RepositoryRoot)
    Assert-ExportPath $Destination $RepositoryRoot @($StageRoot)
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "Required export file is missing: $Source" }
    if (Test-Path -LiteralPath $Destination) { throw "Copy destination already exists: $Destination" }
    New-ExportDirectory (Split-Path -Parent $Destination) $RepositoryRoot $StageRoot
    Copy-Item -LiteralPath $Source -Destination $Destination
}

function Remove-ExportDirectory {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    Assert-ExportPath $Path $RepositoryRoot @($StageRoot)
    if (Test-Path -LiteralPath $Path) {
        Assert-ExportPlainTree $Path
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
}

function Move-ExportDirectory {
    param(
        [Parameter(Mandatory)][string]$Source,
        [Parameter(Mandatory)][string]$Destination,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string[]]$AllowedRoots
    )
    Assert-ExportPath $Source $RepositoryRoot $AllowedRoots
    Assert-ExportPath $Destination $RepositoryRoot $AllowedRoots
    Assert-ExportPlainTree $Source
    if (Test-Path -LiteralPath $Destination) { throw "Move destination already exists: $Destination" }
    if (-not (Test-Path -LiteralPath (Split-Path -Parent $Destination) -PathType Container)) {
        throw "Move destination parent is missing: $Destination"
    }
    Move-Item -LiteralPath $Source -Destination $Destination
}

function Get-ExportTreeFingerprint {
    param([Parameter(Mandatory)][string]$Path)
    Assert-ExportPlainTree $Path
    $fingerprint = [System.Collections.Generic.SortedDictionary[string, string]]::new([System.StringComparer]::Ordinal)
    foreach ($file in Get-ChildItem -LiteralPath $Path -File -Recurse -Force) {
        $relativePath = [System.IO.Path]::GetRelativePath($Path, $file.FullName)
        $fingerprint.Add($relativePath, (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash)
    }
    return $fingerprint
}

function Assert-ExportFingerprint {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)]$Expected
    )
    $actual = Get-ExportTreeFingerprint $Path
    if ($actual.Count -ne $Expected.Count) { throw "Published file count differs from validated output: $Path" }
    foreach ($name in $Expected.Keys) {
        if (-not $actual.ContainsKey($name) -or $actual[$name] -ne $Expected[$name]) {
            throw "Published file differs from validated output: $(Join-Path $Path $name)"
        }
    }
}

function Publish-ExportDirectories {
    param(
        [Parameter(Mandatory)][string]$StagedCode,
        [Parameter(Mandatory)][string]$StagedData,
        [Parameter(Mandatory)][string]$CodeDestination,
        [Parameter(Mandatory)][string]$DataDestination,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    $allowedRoots = @($StageRoot, $CodeDestination, $DataDestination)
    $backupRoot = Join-Path $StageRoot 'previous'
    New-ExportDirectory $backupRoot $RepositoryRoot $StageRoot
    $entries = @(
        @{ Staged = $StagedCode; Live = $CodeDestination; Backup = (Join-Path $backupRoot 'generated'); BackedUp = $false; Published = $false },
        @{ Staged = $StagedData; Live = $DataDestination; Backup = (Join-Path $backupRoot 'bin'); BackedUp = $false; Published = $false }
    )
    foreach ($entry in $entries) {
        Assert-ExportPath $entry.Staged $RepositoryRoot @($StageRoot)
        Assert-ExportPath $entry.Live $RepositoryRoot $allowedRoots
        Assert-ExportPlainTree $entry.Staged
        if (Test-Path -LiteralPath $entry.Live) { Assert-ExportPlainTree $entry.Live }
        $entry.Fingerprint = Get-ExportTreeFingerprint $entry.Staged
    }
    try {
        foreach ($entry in $entries) {
            if (Test-Path -LiteralPath $entry.Live) {
                Move-ExportDirectory $entry.Live $entry.Backup $RepositoryRoot $allowedRoots
                $entry.BackedUp = $true
            }
        }
        foreach ($entry in $entries) {
            Move-ExportDirectory $entry.Staged $entry.Live $RepositoryRoot $allowedRoots
            $entry.Published = $true
        }
        foreach ($entry in $entries) { Assert-ExportFingerprint $entry.Live $entry.Fingerprint }
    } catch {
        $failure = $_
        $rollbackErrors = [System.Collections.Generic.List[string]]::new()
        for ($index = $entries.Count - 1; $index -ge 0; $index--) {
            $entry = $entries[$index]
            if ($entry.Published) {
                try { Move-ExportDirectory $entry.Live $entry.Staged $RepositoryRoot $allowedRoots }
                catch { $rollbackErrors.Add($_.Exception.Message) }
            }
        }
        for ($index = $entries.Count - 1; $index -ge 0; $index--) {
            $entry = $entries[$index]
            if ($entry.BackedUp) {
                try { Move-ExportDirectory $entry.Backup $entry.Live $RepositoryRoot $allowedRoots }
                catch { $rollbackErrors.Add($_.Exception.Message) }
            }
        }
        if ($rollbackErrors.Count -gt 0) {
            $failure.Exception.Data['PreserveStage'] = $true
            $failure.Exception.Data['RollbackErrors'] = [string]::Join([Environment]::NewLine, $rollbackErrors)
        }
        throw $failure
    }
}

function Invoke-ExportProcess {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [Parameter(Mandatory)][string[]]$Arguments,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [switch]$CaptureOutput
    )
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $CaptureOutput.IsPresent
    foreach ($argument in $Arguments) { $startInfo.ArgumentList.Add($argument) }
    $process = [System.Diagnostics.Process]::Start($startInfo)
    if ($null -eq $process) { throw "Could not start export command: $FilePath" }
    try {
        $output = if ($CaptureOutput) { $process.StandardOutput.ReadToEnd() } else { $null }
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) {
            if ($CaptureOutput -and $output) { Write-Host $output }
            $failure = [System.InvalidOperationException]::new("Export command failed with exit code $($process.ExitCode): $FilePath $($Arguments -join ' ')")
            $failure.Data['ProcessExitCode'] = $process.ExitCode
            throw $failure
        }
        if ($CaptureOutput) { return $output }
    } finally {
        $process.Dispose()
    }
}

function ConvertTo-ExportCrlf {
    param(
        [Parameter(Mandatory)][string]$Pattern,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StageRoot
    )
    $utf8 = [System.Text.UTF8Encoding]::new($false)
    foreach ($file in Get-ChildItem -Path $Pattern -File) {
        Assert-ExportPath $file.FullName $RepositoryRoot @($StageRoot)
        $text = [System.IO.File]::ReadAllText($file.FullName, $utf8)
        $crlf = [string]::Join('', @([char]13, [char]10))
        $normalized = [System.Text.RegularExpressions.Regex]::Replace($text, '\r?\n', $crlf)
        if ($normalized -ne $text) {
            [System.IO.File]::WriteAllText($file.FullName, $normalized, $utf8)
        }
    }
}

function Set-ExportDependencyPath {
    param(
        [Parameter(Mandatory)][string]$Cargo,
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$StagedManifest,
        [Parameter(Mandatory)][string]$StageRoot
    )
    $sourceManifest = Join-Path $RepositoryRoot 'Cargo.toml'
    $metadataJson = Invoke-ExportProcess $Cargo @(
        'metadata', '--manifest-path', $sourceManifest,
        '--no-deps', '--format-version', '1', '--offline', '--locked'
    ) $RepositoryRoot -CaptureOutput
    $metadata = $metadataJson | ConvertFrom-Json
    $packages = @($metadata.packages | Where-Object {
        (Get-ExportFullPath $_.manifest_path) -eq (Get-ExportFullPath $sourceManifest)
    })
    if ($packages.Count -ne 1) { throw 'Cargo did not report exactly one game package.' }
    $dependency = @($packages[0].dependencies | Where-Object { $_.name -eq 'bevy_gas' })
    if ($dependency.Count -ne 1) {
        throw 'The game must declare exactly one bevy_gas dependency.'
    }
    $otherLocalDependencies = @($packages[0].dependencies | Where-Object {
        $_.name -ne 'bevy_gas' -and $null -ne $_.PSObject.Properties['path'] -and -not [string]::IsNullOrWhiteSpace($_.path)
    })
    if ($otherLocalDependencies.Count -ne 0) {
        throw 'Export staging currently supports bevy_gas as its only local path dependency.'
    }

    # Git and registry dependencies retain their original source in the temporary manifest.
    if ($null -eq $dependency[0].PSObject.Properties['path'] -or [string]::IsNullOrWhiteSpace($dependency[0].path)) {
        return
    }

    # Cargo reports the dependency path relative to the original manifest as an absolute path.
    # Only the temporary manifest is rewritten; the library sources remain in their own project.
    $dependencyPath = (Get-ExportFullPath $dependency[0].path).Replace('\', '/')
    if (-not (Test-Path -LiteralPath (Join-Path $dependencyPath 'Cargo.toml') -PathType Leaf)) {
        throw "The bevy_gas dependency manifest is missing: $dependencyPath"
    }
    Assert-ExportPath $StagedManifest $RepositoryRoot @($StageRoot)
    $manifestText = Get-Content -LiteralPath $StagedManifest -Raw
    $pattern = '(?m)^(?<prefix>[ \t]*bevy_gas[ \t]*=[ \t]*\{[ \t]*path[ \t]*=[ \t]*)(?:"[^"\r\n]*"|''[^''\r\n]*'')(?<suffix>[^\r\n]*\}[ \t]*)(?=\r?$)'
    $dependencyMatches = [System.Text.RegularExpressions.Regex]::Matches($manifestText, $pattern)
    if ($dependencyMatches.Count -ne 1) {
        throw 'Expected one inline bevy_gas dependency with path as its first field in Cargo.toml.'
    }
    $dependencyMatch = $dependencyMatches[0]
    $replacement = $dependencyMatch.Groups['prefix'].Value + (ConvertTo-Json -InputObject $dependencyPath -Compress) + $dependencyMatch.Groups['suffix'].Value
    $manifestText = $manifestText.Substring(0, $dependencyMatch.Index) + $replacement + $manifestText.Substring($dependencyMatch.Index + $dependencyMatch.Length)
    [System.IO.File]::WriteAllText($StagedManifest, $manifestText, [System.Text.UTF8Encoding]::new($false))
}
$configRoot = [System.IO.Path]::GetFullPath($PSScriptRoot)
$repositoryRoot = Split-Path -Parent $configRoot
$runnerPath = Join-Path $repositoryRoot 'tools/luban/run.ps1'
$configurationPath = Join-Path $configRoot 'luban.conf'
$templateRoot = Join-Path $configRoot 'templates'
$codeOutputPath = Join-Path $configRoot 'generated'
$assetsRoot = Join-Path $repositoryRoot 'assets'
$dataOutputPath = Join-Path $assetsRoot 'config'
$exportRoot = Join-Path $repositoryRoot 'tools/luban/.cache/export'
$stageRoot = Join-Path $exportRoot ([System.Guid]::NewGuid().ToString('N'))
$rawCode = Join-Path $stageRoot 'luban-code'
$stagedData = Join-Path $stageRoot 'data'
$stagedProject = Join-Path $stageRoot 'validation'
$stagedCode = Join-Path $stagedProject 'config/generated'
$stagedManifest = Join-Path $stagedProject 'Cargo.toml'
# Isolate staged package artifacts because compiled code can embed CARGO_MANIFEST_DIR.
$targetDirectory = Join-Path $repositoryRoot 'target/config-export'
$lockStream = $null
$stageCreated = $false
$preserveStage = $false
$exitCode = 1

try {
    foreach ($path in @($runnerPath, $configurationPath, (Join-Path $repositoryRoot 'Cargo.lock'))) {
        Assert-ExportPath $path $repositoryRoot @($repositoryRoot)
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required export file is missing: $path" }
    }
    Assert-ExportPath $templateRoot $repositoryRoot @($configRoot)
    Assert-ExportPlainTree $templateRoot
    foreach ($path in @($configRoot, $codeOutputPath, $dataOutputPath)) {
        Assert-ExportPath $path $repositoryRoot @($configRoot, $dataOutputPath)
        if (Test-Path -LiteralPath $path) { Assert-ExportPlainTree $path }
    }
    New-ExportDirectory $assetsRoot $repositoryRoot $assetsRoot
    Assert-ExportPath $targetDirectory $repositoryRoot @($targetDirectory)
    $cargo = Get-Command cargo -CommandType Application -ErrorAction Stop | Select-Object -First 1 -ExpandProperty Source
    $pwsh = Get-Command pwsh -CommandType Application -ErrorAction Stop | Select-Object -First 1 -ExpandProperty Source
    Assert-ExportPath $exportRoot $repositoryRoot @($exportRoot)
    New-Item -ItemType Directory -Path $exportRoot -Force | Out-Null
    $lockPath = Join-Path $exportRoot '.lock'
    Assert-ExportPath $lockPath $repositoryRoot @($exportRoot)
    try {
        $lockStream = [System.IO.File]::Open($lockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    } catch {
        throw "Could not acquire the configuration export lock. Another export may be running: $lockPath"
    }
    if (Test-Path -LiteralPath $stageRoot) { throw "Export staging directory already exists: $stageRoot" }
    New-ExportDirectory $stageRoot $repositoryRoot $stageRoot
    $stageCreated = $true

    Write-Host 'Generating configuration into an isolated staging directory...'
    Invoke-ExportProcess $pwsh @(
        '-NoProfile', '-File', $runnerPath,
        '--conf', $configurationPath,
        '-t', 'all', '-c', 'rust-bin', '-d', 'bin', '--strict',
        '--customTemplateDir', $templateRoot,
        '-x', "outputCodeDir=$rawCode",
        '-x', "outputDataDir=$stagedData"
    ) $repositoryRoot
    Assert-ExportPlainTree $rawCode
    Assert-ExportPlainTree $stagedData

    # Keep generated Rust modules only; generator-owned package scaffolding stays temporary.
    Copy-ExportDirectory (Join-Path $rawCode 'cfg/src') $stagedCode $repositoryRoot $stageRoot
    $generatedEntry = Join-Path $stagedCode 'lib.rs'
    $moduleEntry = Join-Path $stagedCode 'mod.rs'
    Assert-ExportPath $generatedEntry $repositoryRoot @($stageRoot)
    Assert-ExportPath $moduleEntry $repositoryRoot @($stageRoot)
    if (-not (Test-Path -LiteralPath $generatedEntry -PathType Leaf)) { throw 'The generator did not produce its root Rust module.' }
    if (Test-Path -LiteralPath $moduleEntry) { throw 'The generated module entry already exists.' }
    Move-Item -LiteralPath $generatedEntry -Destination $moduleEntry

    # Validate the candidate using a temporary copy of the same single-package project.
    foreach ($name in @('Cargo.toml', 'Cargo.lock')) {
        Copy-ExportFile (Join-Path $repositoryRoot $name) (Join-Path $stagedProject $name) $repositoryRoot $stageRoot
    }
    Copy-ExportDirectory (Join-Path $repositoryRoot 'src') (Join-Path $stagedProject 'src') $repositoryRoot $stageRoot
    foreach ($name in @('examples', 'tests')) {
        $sourceDirectory = Join-Path $repositoryRoot $name
        if (Test-Path -LiteralPath $sourceDirectory) {
            Copy-ExportDirectory $sourceDirectory (Join-Path $stagedProject $name) $repositoryRoot $stageRoot
        }
    }
    Set-ExportDependencyPath $cargo $repositoryRoot $stagedManifest $stageRoot
    Write-Host 'Formatting and compiling the configuration feature with the generated modules...'
    Invoke-ExportProcess $cargo @('fmt', '--manifest-path', $stagedManifest) $stagedProject
    $buildOutput = Invoke-ExportProcess $cargo @(
        'build', '--manifest-path', $stagedManifest,
        '--features', 'config-validation', '--bin', 'gas-config',
        '--offline', '--locked', '--target-dir', $targetDirectory,
        '--message-format=json-render-diagnostics'
    ) $stagedProject -CaptureOutput
    $executablePaths = [System.Collections.Generic.List[string]]::new()
    foreach ($line in $buildOutput -split '\r?\n') {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        $message = $line | ConvertFrom-Json
        if ($message.reason -eq 'compiler-artifact' -and $message.target.name -eq 'gas-config' -and $message.executable) {
            $executablePaths.Add([string]$message.executable)
        }
    }
    if ($executablePaths.Count -ne 1) { throw 'Cargo did not report exactly one configuration validator executable.' }
    Assert-ExportPath $executablePaths[0] $repositoryRoot @($targetDirectory)
    $validatorPath = Join-Path $stageRoot ('validator/' + [System.IO.Path]::GetFileName($executablePaths[0]))
    Copy-ExportFile $executablePaths[0] $validatorPath $repositoryRoot $stageRoot

    Write-Host 'Writing the package manifest and validating actual binary data and GAS semantics...'
    Invoke-ExportProcess $validatorPath @('write-manifest', $stagedData) $stagedProject
    $manifestPath = Join-Path $stagedData 'manifest.json'
    Assert-ExportPath $manifestPath $repositoryRoot @($stageRoot)
    if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) { throw 'The validator did not produce manifest.json.' }
    Invoke-ExportProcess $validatorPath @('validate-config', $stagedData) $stagedProject

    # rustfmt writes LF line endings, but windows builders expect CRLF under core.autocrlf=true.
    # Normalize published Rust modules back to CRLF before publishing so a regenerate does not
    # show up as a spurious modification on Windows checkouts.
    ConvertTo-ExportCrlf (Join-Path $stagedCode '*.rs') $repositoryRoot $stageRoot

    Write-Host 'Publishing the validated Rust modules and data together...'
    Publish-ExportDirectories $stagedCode $stagedData $codeOutputPath $dataOutputPath $repositoryRoot $stageRoot
    $exitCode = 0
    Write-Host "Configuration export completed: $codeOutputPath and $dataOutputPath"
} catch {
    $failure = $_.Exception
    if ($failure.Data.Contains('ProcessExitCode')) { $exitCode = [int]$failure.Data['ProcessExitCode'] }
    if ($failure.Data.Contains('PreserveStage')) { $preserveStage = [bool]$failure.Data['PreserveStage'] }
    [Console]::Error.WriteLine($failure.Message)
    if ($failure.Data.Contains('RollbackErrors')) {
        [Console]::Error.WriteLine("Automatic rollback needs attention. Backups are preserved in $stageRoot")
        [Console]::Error.WriteLine($failure.Data['RollbackErrors'])
    }
} finally {
    if ($stageCreated -and -not $preserveStage) {
        try { Remove-ExportDirectory $stageRoot $repositoryRoot $stageRoot }
        catch { Write-Warning "Could not remove this export's staging directory: $stageRoot. $($_.Exception.Message)" }
    }
    if ($null -ne $lockStream) { $lockStream.Dispose() }
}
exit $exitCode
