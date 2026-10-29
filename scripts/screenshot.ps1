param(
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$Out,
    [int]$Wait = 5
)
$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class WinApi {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[void][WinApi]::SetProcessDPIAware()

if (-not $env:TODO_LITE_DATA_DIR) { $env:TODO_LITE_DATA_DIR = Join-Path $env:TEMP 'todo-lite-demo' }

$p = Start-Process (Resolve-Path $Exe) -PassThru
try {
    Start-Sleep -Seconds $Wait
    $p.Refresh()
    if ($p.MainWindowHandle -eq [IntPtr]::Zero) { throw 'main window handle not found' }
    $h = $p.MainWindowHandle
    if ([WinApi]::IsIconic($h)) { [void][WinApi]::ShowWindow($h, 9) }
    [void][WinApi]::SetForegroundWindow($h)
    Start-Sleep -Milliseconds 800

    $r = New-Object WinApi+RECT
    [void][WinApi]::GetWindowRect($h, [ref]$r)
    $w = $r.Right - $r.Left
    $ht = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $ht
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $ht))
    $g.Dispose()
    $bmp.Save($Out, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    'saved {0} ({1}x{2}) rect=({3},{4},{5},{6})' -f $Out, $w, $ht, $r.Left, $r.Top, $r.Right, $r.Bottom
}
finally {
    # collect descendants (WebView2 children) before killing the app
    $all = Get-CimInstance Win32_Process
    $ids = New-Object System.Collections.Generic.HashSet[int]
    [void]$ids.Add($p.Id)
    do {
        $before = $ids.Count
        foreach ($proc in $all) { if ($ids.Contains([int]$proc.ParentProcessId)) { [void]$ids.Add([int]$proc.ProcessId) } }
    } while ($ids.Count -gt $before)
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Get-Process msedgewebview2 -ErrorAction SilentlyContinue |
        Where-Object { $ids.Contains($_.Id) } | Stop-Process -Force -ErrorAction SilentlyContinue
}
