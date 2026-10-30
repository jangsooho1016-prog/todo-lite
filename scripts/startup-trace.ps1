param([int]$Runs = 8)
$ErrorActionPreference = 'Stop'

$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\todo-lite.exe'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$base = Join-Path $env:TEMP "todo-lite-trace-$stamp"
New-Item -ItemType Directory $base | Out-Null
$wv = Join-Path $base 'webview'   # reused: run 1 is cold, the rest are warm
$env:TODO_LITE_TIMING = '1'
$env:WEBVIEW2_USER_DATA_FOLDER = $wv

function Get-Num($text, $name) {
    $m = [regex]::Match($text, $name + '=([0-9]+)')
    if ($m.Success) { [double]$m.Groups[1].Value } else { $null }
}
function Get-Mark($text, $name) {
    $m = [regex]::Match($text, '"' + $name + '":([0-9.]+)')
    if ($m.Success) { [double]$m.Groups[1].Value } else { $null }
}
function Stop-Tree($rootId) {
    $all = Get-CimInstance Win32_Process
    $ids = New-Object System.Collections.Generic.HashSet[int]
    [void]$ids.Add($rootId)
    do {
        $n = $ids.Count
        foreach ($p in $all) { if ($ids.Contains([int]$p.ParentProcessId)) { [void]$ids.Add([int]$p.ProcessId) } }
    } while ($ids.Count -gt $n)
    foreach ($id in $ids) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
}

$rows = @()
for ($i = 1; $i -le $Runs; $i++) {
    $data = Join-Path $base "data$i"
    New-Item -ItemType Directory $data | Out-Null
    $env:TODO_LITE_DATA_DIR = $data
    $err = Join-Path $base "stderr$i.txt"
    $out = Join-Path $base "stdout$i.txt"
    $t0 = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
    $proc = Start-Process $exe -PassThru -RedirectStandardError $err -RedirectStandardOutput $out
    $deadline = (Get-Date).AddSeconds(15)
    $text = ''
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 50
        $text = Get-Content $err -Raw -ErrorAction SilentlyContinue
        if ($text -and $text -match 'ready_ms=') { break }
    }
    Start-Sleep -Milliseconds 200
    $text = Get-Content $err -Raw -ErrorAction SilentlyContinue
    Stop-Tree $proc.Id
    Start-Sleep -Milliseconds 800
    if (-not $text -or $text -notmatch 'ready_ms=') { Write-Host "run $i : no ready_ms (see $err)"; continue }

    $runStart = Get-Num $text 't_run_start_epoch_ms'
    $readyEpoch = Get-Num $text 'ready_epoch_ms'
    $rows += [pscustomobject][ordered]@{
        run = $i
        launch_to_run = $runStart - $t0
        db_open = Get-Num $text 't_db_open_ms'
        setup_done = Get-Num $text 't_setup_done_ms'
        page_started = Get-Num $text 't_page_started_ms'
        page_finished = Get-Num $text 't_page_finished_ms'
        js_module = Get-Mark $text 'module'
        js_mounted = Get-Mark $text 'mounted'
        js_synced = Get-Mark $text 'synced_folder'
        js_data = Get-Mark $text 'data'
        js_ready = Get-Mark $text 'ready'
        ready_ms = Get-Num $text 'ready_ms'
        total_wall = $readyEpoch - $t0
    }
}

'Columns (ms): launch_to_run = Start-Process -> run() start (epoch diff); db_open/setup_done/page_*/ready_ms = since run() start;'
'js_* = performance.now() since WebView navigation start; total_wall = Start-Process -> ready (epoch diff).'
''
'--- run 1 (cold WebView2 profile) ---'
$rows | Select-Object -First 1 | Format-Table -AutoSize | Out-String -Width 250
'--- warm runs ---'
$warm = $rows | Select-Object -Skip 1
$warm | Format-Table -AutoSize | Out-String -Width 250
'--- warm mean / min / max ---'
$cols = $rows[0].PSObject.Properties.Name | Where-Object { $_ -ne 'run' }
$stats = foreach ($c in $cols) {
    $v = $warm | ForEach-Object { $_.$c } | Where-Object { $_ -ne $null }
    if ($v) {
        $m = $v | Measure-Object -Average -Minimum -Maximum
        [pscustomobject]@{ column = $c; mean = [math]::Round($m.Average, 1); min = $m.Minimum; max = $m.Maximum }
    }
}
$stats | Format-Table -AutoSize | Out-String -Width 250
"raw logs: $base"
