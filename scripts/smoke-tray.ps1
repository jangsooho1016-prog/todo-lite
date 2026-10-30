# 트레이 상주 + 빠른 입력창 스모크 검증. 화면에 키/마우스 입력을 보내지 않고, 이 스크립트가 띄운 프로세스의
# 창만 다룬다(PID로 거른다).
#   .\scripts\smoke-tray.ps1                       # target\release\todo-lite.exe
#   .\scripts\smoke-tray.ps1 -Exe target\debug\todo-lite.exe -Shot quick.png
param(
    [string]$Exe = 'target\release\todo-lite.exe',
    [string]$Shot = ''
)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
if (-not [System.IO.Path]::IsPathRooted($Exe)) { $Exe = Join-Path $root $Exe }
if (-not (Test-Path $Exe)) { throw "실행 파일이 없어요: $Exe" }
if ($Shot -and -not [System.IO.Path]::IsPathRooted($Shot)) { $Shot = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Shot)) }

$MainTitle = 'Todo-Lite'
$QuickTitle = 'Todo-Lite 빠른 입력'

if (-not ('SmokeWin' -as [type])) {
    Add-Type -AssemblyName System.Drawing
    Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class SmokeWin {
    delegate bool EnumProc(IntPtr h, IntPtr l);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr FindWindowEx(IntPtr parent, IntPtr after, string cls, string title);
    [DllImport("user32.dll")] static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);

    // 제목이 정확히 title인 이 프로세스의 최상위 창. visibleOnly이면 보이는 창만.
    static IntPtr Find(uint pid, string title, bool visibleOnly) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p != pid) return true;
            if (visibleOnly && !IsWindowVisible(h)) return true;
            var s = new StringBuilder(256); GetWindowText(h, s, 256);
            if (s.ToString() == title) found = h;
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static bool IsVisible(uint pid, string title) { return Find(pid, title, true) != IntPtr.Zero; }
    public static bool PostClose(uint pid, string title) {
        IntPtr h = Find(pid, title, false);
        return h != IntPtr.Zero && PostMessage(h, 0x0010, IntPtr.Zero, IntPtr.Zero);
    }
    public static int[] Rect(uint pid, string title) {
        IntPtr h = Find(pid, title, true);
        if (h == IntPtr.Zero) return null;
        RECT r; GetWindowRect(h, out r);
        return new int[] { r.L, r.T, r.R - r.L, r.B - r.T };
    }
    // 창 클래스가 cls인 이 프로세스의 창(최상위 또는 메시지 전용 창)이 있는가.
    public static bool HasClass(uint pid, string cls) {
        bool found = false;
        EnumWindows((h, l) => {
            uint p; GetWindowThreadProcessId(h, out p);
            if (p != pid) return true;
            var c = new StringBuilder(256); GetClassName(h, c, 256);
            if (c.ToString() == cls) found = true;
            return true;
        }, IntPtr.Zero);
        if (found) return true;
        IntPtr msgParent = new IntPtr(-3); // HWND_MESSAGE
        IntPtr w = IntPtr.Zero;
        while ((w = FindWindowEx(msgParent, w, cls, null)) != IntPtr.Zero) {
            uint p; GetWindowThreadProcessId(w, out p);
            if (p == pid) return true;
        }
        return false;
    }
}
'@
}

$script:total = 0
$script:failed = 0
function Check([string]$name, [bool]$ok) {
    $script:total++
    if (-not $ok) { $script:failed++ }
    '{0}  {1}' -f $(if ($ok) { 'PASS' } else { 'FAIL' }), $name
}

$ts = Get-Date -Format 'yyyyMMdd-HHmmss'
$tmp = Join-Path $env:TEMP "todo-lite-smoke-$ts"
New-Item -ItemType Directory $tmp | Out-Null
$errFile = Join-Path $tmp 'stderr.txt'
$outFile = Join-Path $tmp 'stdout.txt'
$saved = @{}
foreach ($n in 'TODO_LITE_DATA_DIR', 'TODO_LITE_TIMING', 'TODO_LITE_AUTOTEST_QUICK') { $saved[$n] = [Environment]::GetEnvironmentVariable($n) }
$env:TODO_LITE_DATA_DIR = Join-Path $tmp 'data'
$env:TODO_LITE_TIMING = '1'
$env:TODO_LITE_AUTOTEST_QUICK = '1'

$p = $null
try {
    '--- 실행: {0}' -f $Exe
    $p = Start-Process $Exe -PassThru -RedirectStandardError $errFile -RedirectStandardOutput $outFile
    $procId = [uint32]$p.Id

    # 빠른 창이 보일 때까지(최대 15초)
    $sw = [Diagnostics.Stopwatch]::StartNew()
    while ($sw.Elapsed.TotalSeconds -lt 15 -and -not $p.HasExited -and -not [SmokeWin]::IsVisible($procId, $QuickTitle)) {
        Start-Sleep -Milliseconds 200
    }
    $p.Refresh()

    '--- 시작 후 상태 ({0:N1}초)' -f $sw.Elapsed.TotalSeconds
    Check '프로세스가 살아 있음' (-not $p.HasExited)
    Check "메인 창 '$MainTitle' 보임" ([SmokeWin]::IsVisible($procId, $MainTitle))
    Check "빠른 창 '$QuickTitle' 보임" ([SmokeWin]::IsVisible($procId, $QuickTitle))
    Check '트레이 창(tray_icon_app) 있음' ([SmokeWin]::HasClass($procId, 'tray_icon_app'))
    Check '전역 단축키 창(global_hotkey_app) 있음' ([SmokeWin]::HasClass($procId, 'global_hotkey_app'))

    if ($Shot) {
        $rect = [SmokeWin]::Rect($procId, $QuickTitle)
        if ($rect) {
            Start-Sleep -Milliseconds 500   # 화면이 그려질 시간
            $rect = [SmokeWin]::Rect($procId, $QuickTitle)
            $bmp = New-Object System.Drawing.Bitmap $rect[2], $rect[3]
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.CopyFromScreen($rect[0], $rect[1], 0, 0, $bmp.Size)
            $bmp.Save($Shot, [System.Drawing.Imaging.ImageFormat]::Png)
            $g.Dispose(); $bmp.Dispose()
            '스크린샷(빠른 창만): {0}  rect={1}' -f $Shot, ($rect -join ',')
        } else { '스크린샷을 건너뜀: 빠른 창을 찾지 못함' }
    }

    # X 버튼과 같은 종료 요청을 메인 창에만 보낸다. (Process.CloseMainWindow()는 빠른 창이 뜬 뒤에는 그쪽을 고를 수 있다.)
    '--- 메인 창 닫기 요청 (트레이로 숨겨져야 함)'
    [void][SmokeWin]::PostClose($procId, $MainTitle)
    Start-Sleep -Seconds 3
    $p.Refresh()
    Check '프로세스가 계속 살아 있음' (-not $p.HasExited)
    Check '메인 창이 숨겨짐' (-not [SmokeWin]::IsVisible($procId, $MainTitle))

    '--- 빠른 창 닫기 요청 (숨겨져야 함)'
    [void][SmokeWin]::PostClose($procId, $QuickTitle)
    Start-Sleep -Seconds 1
    $p.Refresh()
    Check '프로세스가 계속 살아 있음' (-not $p.HasExited)
    Check '빠른 창이 숨겨짐' (-not [SmokeWin]::IsVisible($procId, $QuickTitle))
}
finally {
    # 프로세스 트리 정리 (이 스크립트가 띄운 것만)
    if ($p) {
        $all = Get-CimInstance Win32_Process
        $ids = New-Object System.Collections.Generic.HashSet[int]
        [void]$ids.Add($p.Id)
        do {
            $before = $ids.Count
            foreach ($proc in $all) { if ($ids.Contains([int]$proc.ParentProcessId)) { [void]$ids.Add([int]$proc.ProcessId) } }
        } while ($ids.Count -gt $before)
        foreach ($id in $ids) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
        Start-Sleep -Milliseconds 500
    }
    foreach ($n in $saved.Keys) { [Environment]::SetEnvironmentVariable($n, $saved[$n]) }
}

'--- 시간 (stderr: {0})' -f $errFile
$lines = if (Test-Path $errFile) { Get-Content $errFile } else { @() }
function Mark([string]$pattern) {
    foreach ($l in $lines) { $m = [regex]::Match($l, $pattern); if ($m.Success) { return [int]$m.Groups[1].Value } }
    return $null
}
$ready = Mark '\bready_ms=(\d+)'
$trigger = Mark '^t_quick_trigger_ms=(\d+)'
$shown = Mark '^t_quick_shown_ms=(\d+)'
$pageDone = Mark '^t_page_finished_ms=(\d+).*label=quick'
function Show([string]$name, $v) { '{0} = {1}' -f $name, $(if ($null -ne $v) { $v } else { '(없음)' }) }
Show 'ready_ms' $ready
Show 't_quick_trigger_ms' $trigger
Show 't_quick_shown_ms' $shown
Show 't_page_finished_ms (quick)' $pageDone
Show 'quick_trigger_to_shown_ms' $(if ($null -ne $trigger -and $null -ne $shown) { $shown - $trigger })
Show 'quick_trigger_to_page_finished_ms' $(if ($null -ne $trigger -and $null -ne $pageDone) { $pageDone - $trigger })

''
'SMOKE: {0}/{1} passed' -f ($script:total - $script:failed), $script:total
if ($script:failed -gt 0) { exit 1 }
