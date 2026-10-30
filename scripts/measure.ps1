param([int]$IdleSeconds = 20)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\todo-lite.exe'
$installer = Get-ChildItem (Join-Path $root 'target\release\bundle\nsis\*.exe') |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1

'--- 설치 파일 ---'
'{0}: {1:N1} MB (목표 < 50 MB)' -f $installer.Name, ($installer.Length / 1MB)

$tmp = Join-Path $env:TEMP 'todo-lite-measure'
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $tmp | Out-Null
$env:TODO_LITE_DATA_DIR = $tmp
$env:TODO_LITE_TIMING = '1'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $tmp 'webview'   # 매번 콜드 프로필
$errFile = Join-Path $tmp 'stderr.txt'

$p = Start-Process $exe -PassThru -RedirectStandardError $errFile
Start-Sleep -Seconds $IdleSeconds

# 앱과 그 자식(WebView2) 프로세스를 모은다
$all = Get-CimInstance Win32_Process
$ids = New-Object System.Collections.Generic.HashSet[int]
[void]$ids.Add($p.Id)
do {
    $before = $ids.Count
    foreach ($proc in $all) { if ($ids.Contains([int]$proc.ParentProcessId)) { [void]$ids.Add([int]$proc.ProcessId) } }
} while ($ids.Count -gt $before)

$procs = Get-Process -Id ($ids | ForEach-Object { $_ }) -ErrorAction SilentlyContinue
$ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum
$commit = ($procs | Measure-Object PrivateMemorySize64 -Sum).Sum
$perf = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process |
    Where-Object { $ids.Contains([int]$_.IDProcess) }
$wsPrivate = ($perf | Measure-Object WorkingSetPrivate -Sum).Sum

'--- 유휴 메모리 ({0}초 후, 프로세스 {1}개, WebView2 포함) ---' -f $IdleSeconds, @($procs).Count
'프라이빗 작업 집합 합계 (목표 약 100 MB 이하): {0:N0} MB' -f ($wsPrivate / 1MB)
'작업 집합 합계 (공유 페이지 중복 계산됨): {0:N0} MB' -f ($ws / 1MB)
'커밋(프라이빗 바이트) 합계: {0:N0} MB' -f ($commit / 1MB)

'--- 시작 시간 ---'
$ready = Select-String -Path $errFile -Pattern 'ready_ms=(\d+)' | Select-Object -First 1
if ($ready) { 'ready_ms = {0} (절대값 참고용. 목표는 빈 Tauri 앱 대비 +0.5초 이내)' -f $ready.Matches[0].Groups[1].Value } else { 'ready_ms 출력을 찾지 못함 (stderr: ' + $errFile + ')' }

Stop-Process -Id $p.Id -Force
Get-Process msedgewebview2 -ErrorAction SilentlyContinue |
    Where-Object { $ids.Contains($_.Id) } | Stop-Process -Force -ErrorAction SilentlyContinue
