# Spawned by src-tauri/src/updater.rs to finish a Windows update after the
# main Luma process has exited. Windows won't let the update replace the
# running .exe directly (it's locked while executing), so instead Luma
# downloads the new build to a ".update" file next to itself, spawns this
# script detached, and exits - this script waits for that process to
# actually be gone, swaps the files, and relaunches the new build.
#
# __PID__, __OLD__, __NEW__, __MARKER__, __LOG__ are replaced with real
# values before this is written out and run - see install_and_relaunch()
# in updater.rs.
#
# This process (the parent Luma) has *already exited* (or is about to) by
# the time this runs, so there is no live Tauri command left to report
# back to if the swap fails - __MARKER__ is the only channel left for
# that, checked by take_last_update_failure() the next time Luma starts.

$oldPath = "__OLD__"
$newPath = "__NEW__"
$markerPath = "__MARKER__"
$logPath = "__LOG__"
# Not named $pid - that's PowerShell's reserved automatic variable for
# *this* process and can't be reassigned.
$targetPid = __PID__

function Write-Log($message) {
    try {
        $line = "[{0}] {1}" -f (Get-Date -Format "yyyy-MM-dd HH:mm:ss.fff"), $message
        Add-Content -Path $logPath -Value $line -ErrorAction SilentlyContinue
    } catch {
        # Logging is best-effort - never let a logging failure break the
        # actual update.
    }
}

Write-Log "helper started for pid $targetPid ('$oldPath' <- '$newPath')"

try {
    Wait-Process -Id $targetPid -Timeout 30 -ErrorAction SilentlyContinue
} catch {
    # Already gone, or Wait-Process itself isn't happy - either way, fall
    # through to the retry loop below rather than giving up here.
    Write-Log "Wait-Process threw: $_"
}

# The OS - and on a fresh, unsigned .exe, often Windows Defender/
# SmartScreen doing a reputation check - can hold a lock on the file for
# well longer than a couple of seconds even after the process object is
# gone, so this retries for up to ~30s rather than giving up fast.
$attempts = 0
$maxAttempts = 60
$swapped = $false
$lastError = $null

while ($attempts -lt $maxAttempts -and -not $swapped) {
    try {
        # A single Move-Item -Force, not a separate Remove-Item followed
        # by Move-Item: if a prior version of this script deleted
        # $oldPath and *then* the move failed, every retry after that
        # point failed too (Remove-Item on a path that's already gone
        # throws), leaving neither the old nor the new file in place.
        # Move-Item -Force replaces the destination atomically in one
        # step, so a failed attempt always leaves both files exactly
        # where they were and the next retry starts from a clean state.
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
        # If even writing the marker fails, there's nothing more this
        # script can do to report it.
    }
    # Never leave the user with no running app at all just because the
    # swap failed - the old build is still sitting right there and still
    # works.
    if (Test-Path $oldPath) {
        Start-Process -FilePath $oldPath
    }
}

# Best-effort cleanup of this script itself - PowerShell has already read
# the whole file into memory to run it, so deleting it out from under
# itself here is safe and doesn't need to wait for anything.
Remove-Item -Path $PSCommandPath -Force -ErrorAction SilentlyContinue
