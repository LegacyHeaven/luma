# Spawned by src-tauri/src/updater.rs to finish a Windows update after the
# main Luma process has exited. Windows won't let the update replace the
# running .exe directly (it's locked while executing), so instead Luma
# downloads the new build to a ".update" file next to itself, spawns this
# script detached, and exits - this script waits for that process to
# actually be gone, swaps the files, and relaunches the new build.
#
# __PID__, __OLD__, __NEW__ are replaced with real values before this is
# written out and run - see install_and_relaunch() in updater.rs.

$oldPath = "__OLD__"
$newPath = "__NEW__"
# Not named $pid - that's PowerShell's reserved automatic variable for
# *this* process and can't be reassigned.
$targetPid = __PID__

try {
    Wait-Process -Id $targetPid -Timeout 30 -ErrorAction SilentlyContinue
} catch {
    # Already gone, or Wait-Process itself isn't happy - either way, fall
    # through to the retry loop below rather than giving up here.
}

# The OS can take a moment to actually release the file handle even after
# the process object is gone, so retry the swap instead of failing on the
# first locked-file error.
$attempts = 0
$swapped = $false
while ($attempts -lt 20 -and -not $swapped) {
    try {
        Remove-Item -Path $oldPath -Force -ErrorAction Stop
        Move-Item -Path $newPath -Destination $oldPath -Force -ErrorAction Stop
        $swapped = $true
    } catch {
        Start-Sleep -Milliseconds 300
        $attempts++
    }
}

if ($swapped) {
    Start-Process -FilePath $oldPath
}
