[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$BootstrapArchive,

    [Parameter(Mandatory = $true)]
    [string]$BootstrapCommit,

    [Parameter(Mandatory = $true)]
    [string]$CandidateCommit,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedCandidateVersion
)

$ErrorActionPreference = "Stop"

if ($BootstrapCommit -notmatch '^[0-9a-f]{40}$' -or
    $CandidateCommit -notmatch '^[0-9a-f]{40}$' -or
    $BootstrapCommit -eq $CandidateCommit -or
    $ExpectedCandidateVersion -notmatch '^[0-9A-Za-z][0-9A-Za-z.+-]{0,63}$') {
    throw "qualification requires two distinct lowercase 40-hex commits and a valid candidate version"
}
if (-not (Test-Path -LiteralPath $BootstrapArchive -PathType Leaf) -or
    -not (Test-Path -LiteralPath "$BootstrapArchive.sha256" -PathType Leaf)) {
    throw "bootstrap archive or checksum is missing"
}

$archiveName = Split-Path -Leaf $BootstrapArchive
$checksumLine = (Get-Content -LiteralPath "$BootstrapArchive.sha256" -Raw).Trim()
$checksumParts = $checksumLine -split '  ', 2
if ($checksumParts.Count -ne 2 -or $checksumParts[1] -ne $archiveName) {
    throw "bootstrap checksum has an invalid archive name"
}
$archiveDigest = (Get-FileHash -LiteralPath $BootstrapArchive -Algorithm SHA256).Hash.ToLowerInvariant()
if ($checksumParts[0] -cne $archiveDigest) {
    throw "bootstrap archive checksum mismatch"
}

$qualificationRoot = Join-Path $env:RUNNER_TEMP ("skm-release-update-" + [guid]::NewGuid().ToString("N"))
$installDir = Join-Path $qualificationRoot "install"
New-Item -ItemType Directory -Path $installDir -Force | Out-Null

try {
    Expand-Archive -LiteralPath $BootstrapArchive -DestinationPath $installDir
    $skmPath = Join-Path $installDir "skm.exe"
    if (-not (Test-Path -LiteralPath $skmPath -PathType Leaf)) {
        throw "bootstrap archive did not contain skm.exe"
    }

    $env:SKM_NO_UPDATE_CHECK = "1"
    $bootstrapVersion = (& $skmPath version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or
        $bootstrapVersion -notmatch '^skm ([^ ]+) \(development - ([0-9a-f]{40})\)$' -or
        $Matches[2] -cne $BootstrapCommit) {
        throw "unexpected bootstrap identity: $bootstrapVersion"
    }
    $bootstrapDigest = (Get-FileHash -LiteralPath $skmPath -Algorithm SHA256).Hash.ToLowerInvariant()

    Remove-Item Env:SKM_NO_UPDATE_CHECK
    . (Join-Path $PSScriptRoot "invoke-windows-pseudoterminal.ps1")

    $candidateShort = $CandidateCommit.Substring(0, 7)
    $noticeSeen = $false
    $noticeOutput = ""
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        $noticeResult = Invoke-WindowsPseudoTerminal `
            -Executable $skmPath `
            -Arguments @("version") `
            -TimeoutMilliseconds 60000
        $noticeOutput = $noticeResult.Output
        if ($noticeResult.ExitCode -eq 0 -and
            $noticeOutput.Contains('[skm] Channel update available:') -and
            $noticeOutput.Contains($candidateShort)) {
            $noticeSeen = $true
            break
        }
    }
    if (-not $noticeSeen) {
        throw "bootstrap binary did not emit the candidate update notice in a terminal: $noticeOutput"
    }

    $updateOutput = (& $skmPath update 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or
        -not $updateOutput.Contains('Updated skm:') -or
        -not $updateOutput.Contains($BootstrapCommit.Substring(0, 7)) -or
        -not $updateOutput.Contains($candidateShort)) {
        throw "self-update failed or reported the wrong revisions: $updateOutput"
    }
    Write-Output $updateOutput

    $env:SKM_NO_UPDATE_CHECK = "1"
    $candidateIdentity = (& $skmPath version | Out-String).Trim()
    $expectedCandidate = "skm $ExpectedCandidateVersion (development - $CandidateCommit)"
    if ($LASTEXITCODE -ne 0 -or $candidateIdentity -cne $expectedCandidate) {
        throw "updated executable has the wrong identity: $candidateIdentity"
    }
    $candidateDigest = (Get-FileHash -LiteralPath $skmPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($candidateDigest -ceq $bootstrapDigest) {
        throw "update did not replace the bootstrap executable bytes"
    }

    $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(60)
    do {
        $updateDebris = @(
            Get-ChildItem -LiteralPath $installDir -Force |
                Where-Object { $_.Name.StartsWith('.skm-update-', [StringComparison]::Ordinal) }
        )
        if ($updateDebris.Count -eq 0) {
            break
        }
        Start-Sleep -Milliseconds 200
    } while ([DateTime]::UtcNow -lt $cleanupDeadline)
    $updateDebris = @(
        Get-ChildItem -LiteralPath $installDir -Force |
            Where-Object { $_.Name.StartsWith('.skm-update-', [StringComparison]::Ordinal) }
    )
    if ($updateDebris.Count -ne 0) {
        $debrisNames = ($updateDebris | ForEach-Object Name) -join ', '
        throw "Windows self-update cleanup did not converge: $debrisNames"
    }

    Remove-Item Env:SKM_NO_UPDATE_CHECK
    $noopOutput = (& $skmPath update 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or
        -not $noopOutput.Contains('skm is already up to date:') -or
        -not $noopOutput.Contains($candidateShort)) {
        throw "already-current update failed: $noopOutput"
    }
    Write-Output $noopOutput
    $noopDigest = (Get-FileHash -LiteralPath $skmPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($noopDigest -cne $candidateDigest) {
        throw "already-current update changed the executable"
    }
} finally {
    Remove-Item Env:SKM_NO_UPDATE_CHECK -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $qualificationRoot -Recurse -Force -ErrorAction SilentlyContinue
}

