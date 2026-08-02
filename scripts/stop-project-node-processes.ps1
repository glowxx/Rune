[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $ProjectRoot
)

$ErrorActionPreference = 'Stop'
$null = Resolve-Path -LiteralPath $ProjectRoot
$devPort = 1420

try {
    # netstat exposes listener ownership without requiring the elevated CIM
    # permissions needed to read another process's full command line.
    $listenerPids = @(
        netstat.exe -ano -p TCP |
            ForEach-Object {
                if ($_ -match "^\s*TCP\s+\S+:$devPort\s+\S+\s+LISTENING\s+(\d+)\s*$") {
                    [int] $Matches[1]
                }
            } |
            Sort-Object -Unique
    )

    $projectNodeProcesses = @(
        foreach ($processId in $listenerPids) {
            $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
            if ($process -and $process.ProcessName -eq 'node') {
                $process
            }
        }
    )

    if ($projectNodeProcesses.Count -eq 0) {
        Write-Host "[preflight] No Node process is listening on Rune dev port $devPort."
        exit 0
    }

    foreach ($process in $projectNodeProcesses) {
        Write-Host "[preflight] Stopping Rune dev server PID $($process.Id)..."
        Stop-Process -Id $process.Id -Force -ErrorAction Stop
    }

    # Native DLL handles can remain briefly pending after process termination.
    Start-Sleep -Milliseconds 750
    Write-Host "[preflight] Stopped $($projectNodeProcesses.Count) Rune Node process(es)."
}
catch {
    Write-Error "Failed to release Rune Node processes: $($_.Exception.Message)"
    exit 1
}
