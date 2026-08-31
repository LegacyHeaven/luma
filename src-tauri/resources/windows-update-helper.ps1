
$oldPath = "__OLD__"
$newPath = "__NEW__"
$markerPath = "__MARKER__"
$logPath = "__LOG__"
$targetPid = __PID__

function Write-Log($message) {
    try {
        $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"), $message
        Add-Content -Path $logPath -Value $line -ErrorAction SilentlyContinue
    } catch {
    }
}

Write-Log "helper started for pid $targetPid ('$oldPath' <- '$newPath')"

try {
    Wait-Process -Id $targetPid -Timeout 30 -ErrorAction SilentlyContinue
} catch {
    Write-Log "Wait-Process threw: $_"
}

$attempts = 0
$maxAttempts = 60
$swapped = $false
$lastError = $null

while ($attempts -lt $maxAttempts -and -not $swapped) {
    try {
        Move-Item -Path $newPath -Destination $oldPath -Force -ErrorAction Stop
        $swapped = $true
    } catch {
        $lastError = $_.Exception.Message
        Start-Sleep -Milliseconds 500
        $attempts++
    }
}

if ($swapped) {
    Write-Log "swap succeeded after $attempts retries, relaunching"
    Remove-Item -Path $markerPath -Force -ErrorAction SilentlyContinue
    Start-Process -FilePath $oldPath
} else {
    Write-Log "swap FAILED after $attempts retries - last error: $lastError"
    try {
        "The last automatic update couldn't finish - LUMA is still on the version you had. ($lastError)" |
            Set-Content -Path $markerPath -ErrorAction SilentlyContinue
    } catch {
    }
    if (Test-Path $oldPath) {
        Start-Process -FilePath $oldPath
    }
}

Remove-Item -Path $PSCommandPath -Force -ErrorAction SilentlyContinue
