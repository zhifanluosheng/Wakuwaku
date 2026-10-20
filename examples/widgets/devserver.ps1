# Your dev servers in Wakuwaku's island, found by itself: every local port a
# dev tool is listening on (Vite, Next, Nuxt, node, dotnet, uvicorn, Flask,
# rails, php, go, bun, deno...), one widget each ("web 5173 ?"), so several
# show at once. One that was up and goes down is kept, grey, as 已停止 for a
# day, and -Start brings it back (from the terminal:
# `pwsh devserver.ps1 -Start 5173`; the command it was seen with is kept in
# her data folder, devservers.json).
#
#   pwsh examples/widgets/devserver.ps1                  # find, and keep finding
#   pwsh examples/widgets/devserver.ps1 -Once            # one look
#   pwsh examples/widgets/devserver.ps1 -Url http://localhost:3000,http://localhost:5173/api/health
#   pwsh examples/widgets/devserver.ps1 -Start 5173      # start one kept as stopped
#   pwsh examples/widgets/devserver.ps1 -Names bun,deno  # more program names to count
[CmdletBinding(PositionalBinding = $false)]
param(
  [string[]]$Url = @(),
  [string[]]$Names = @(),
  [int]$Seconds = 10,
  [int]$Port = 47213,
  [string[]]$Start = @(),
  [switch]$Once
)

# From cmd or bash, "-Url a,b" and "-Names bun,deno" come in as one word.
$Url = @($Url | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$Names = @($Names | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })
$Start = @($Start | ForEach-Object { $_ -split ',' } | ForEach-Object { $_.Trim() } | Where-Object { $_ })

# Beside the plugins (her data folder), or beside this file when run by hand.
$data = if ($env:WAKUWAKU_USER_DATA) { $env:WAKUWAKU_USER_DATA }
  elseif ((Split-Path -Leaf $PSScriptRoot) -eq 'plugins') { Split-Path -Parent $PSScriptRoot }
  else { $PSScriptRoot }
$stateFile = Join-Path $data 'devservers.json'

function Send($widget) {
  $body = $widget | ConvertTo-Json -Compress -Depth 6
  try {
    Invoke-RestMethod -Method Post "http://127.0.0.1:$Port/widget" -ContentType 'application/json; charset=utf-8' -Body ([Text.Encoding]::UTF8.GetBytes($body)) -TimeoutSec 5 | Out-Null
  } catch { }
}

# The few characters a server goes by: its port.
function Short($u) {
  $uri = [uri]$u
  if ($uri.IsDefaultPort) { $uri.Host } else { "$($uri.Port)" }
}

function IsUp($u) {
  try {
    Invoke-WebRequest $u -TimeoutSec 3 -UseBasicParsing -MaximumRedirection 0 -ErrorAction Stop | Out-Null
    return $true
  } catch {
    return [bool]$_.Exception.Response
  }
}

# Programs that run a dev server (the command line settles the rest).
$DEV_NAMES = @(
  'node', 'npm', 'pnpm', 'yarn', 'bun', 'deno', 'vite', 'next', 'nuxt',
  'python', 'pythonw', 'uvicorn', 'gunicorn', 'flask', 'dotnet',
  'ruby', 'rails', 'php', 'go', 'cargo', 'air', 'hugo', 'mkdocs', 'jekyll',
  'http-server', 'live-server', 'webpack', 'esbuild', 'parcel', 'turbo', 'nx'
) + $Names

# Not a dev server, however it looks: Windows itself, her, and other apps.
$SKIP_NAMES = @(
  'System', 'Idle', 'svchost', 'lsass', 'wininit', 'services', 'spoolsv',
  'dllhost', 'explorer', 'SearchIndexer', 'MsMpEng', 'SecurityHealthService',
  'smss', 'csrss', 'winlogon', 'fontdrvhost', 'ctfmon', 'sihost', 'taskhostw',
  'RuntimeBroker', 'ShellExperienceHost', 'StartMenuExperienceHost',
  'SearchHost', 'msedge', 'chrome', 'firefox', 'Discord', 'Spotify', 'Code',
  'devenv', 'clash', 'Clash', 'mihomo', 'Weixin', 'WeChat', 'Telegram', 'QQ', 'msedgewebview2',
  'Time', 'OpenCode', 'orca', 'wakuwaku', 'sshd', 'nginx', 'httpd', 'mysqld', 'postgres',
  'redis-server', 'mongod', 'docker', 'com.docker.backend'
)

# What the process says it runs: node_modules/.bin/vite, next dev, uvicorn...
$DEV_LINE = '(?i)(vite|next\s+dev|next-server|nuxt\s+dev|npm(-cli\.js)?\s+(run\s+)?(dev|start|serve)|pnpm(-cli\.js)?\s+(run\s+)?(dev|start|serve)|yarn(-cli\.js)?\s+(dev|start|serve)|bun\s+(run\s+)?(dev|start)|deno\s+(run|task)|flask\s+run|uvicorn|gunicorn|dotnet\s+(run|watch)|go\s+run|cargo\s+(run|watch)|rails\s+(s|server)|php\s+-S|hugo\s+(s|server)|mkdocs\s+serve|jekyll\s+serve|webpack\s+serve|parcel|ng\s+serve|vue-cli-service\s+serve|react-scripts\s+start|nodemon|ts-node|tsx\s+watch)'

# The port a dev tool usually takes, when its process name says nothing
# (a wrapper started as "node" on 3000, a Go binary on 8080).
$DEV_PORTS = @(3000, 3001, 3002, 4000, 4200, 4321, 5000, 5001, 5173, 5174, 5500, 5555,
  8000, 8001, 8080, 8081, 8082, 8888, 9000, 9001, 9090, 9091, 1313, 1420, 3333,
  4001, 4173, 5002, 5050, 7777)

function ReadState {
  try {
    if (Test-Path $stateFile) {
      $raw = Get-Content $stateFile -Raw -Encoding UTF8
      if ($raw.Trim()) { return @($raw | ConvertFrom-Json) }
    }
  } catch { }
  return @()
}

function WriteState($list) {
  try {
    $json = ConvertTo-Json -InputObject @($list) -Depth 6 -Compress
    [IO.File]::WriteAllText($stateFile, $json, (New-Object Text.UTF8Encoding $false))
  } catch { }
}

# A path the command line names, as a file that is there.
function PathsIn($cmd) {
  if (-not $cmd) { return @() }
  $out = @()
  foreach ($m in [regex]::Matches($cmd, '''([^'']+)''|"([^"]+)"|([A-Za-z]:\\[^\s"'']+\.(js|mjs|cjs|ts|py|rb|php|json|toml)|[^\s"'']+\.(js|mjs|cjs|ts|py|rb|php))')) {
    $p = $m.Groups[1].Value + $m.Groups[2].Value + $m.Groups[3].Value
    if ($p -and (Test-Path -LiteralPath $p)) { $out += $p }
  }
  return @($out | Select-Object -Unique)
}

# Where a process's project is: the folder of a script it runs, up to the
# nearest package.json / pyproject.toml / .git; else its own program's folder.
function ProjectOf($cmd, $exe) {
  foreach ($p in (PathsIn $cmd)) {
    $full = try { (Resolve-Path -LiteralPath $p -ErrorAction Stop).Path } catch { $p }
    $dir = if (Test-Path -LiteralPath $full -PathType Container) { $full } else { Split-Path -Parent $full }
    for ($i = 0; $i -lt 8 -and $dir; $i++) {
      if ((Split-Path -Leaf $dir) -eq 'node_modules') { break }
      foreach ($mark in 'package.json', 'pyproject.toml', 'requirements.txt', 'go.mod', 'Cargo.toml', 'composer.json', '.git') {
        if (Test-Path -LiteralPath (Join-Path $dir $mark)) { return $dir }
      }
      $next = Split-Path -Parent $dir
      if ($next -eq $dir) { break }
      $dir = $next
    }
  }
  if ($exe) {
    $dir = Split-Path -Parent $exe
    # Not the folder a tool is installed in.
    if ($dir -notmatch '(?i)(\\Program Files|\\Windows\\|\\AppData\\Local\\Programs\\|\\nodejs$|\\.npm$|\\.bun$|\\.deno$)') { return $dir }
  }
  return ''
}

# What is listening on this machine that looks like a dev server:
# a list of { port; name; cmd; exe; project }.
function Discover {
  $found = @{}
  try {
    $listens = Get-NetTCPConnection -State Listen -ErrorAction Stop | Where-Object { $_.LocalPort -ge 1024 -and $_.LocalPort -ne $Port }
  } catch { return @() }
  foreach ($l in $listens) {
    $procId = $l.OwningProcess
    if (-not $procId) { continue }
    try { $proc = Get-Process -Id $procId -ErrorAction Stop } catch { continue }
    $name = $proc.ProcessName
    if ($SKIP_NAMES -contains $name) { continue }
    $cmd = ''
    $exe = ''
    try {
      $w = Get-CimInstance Win32_Process -Filter "ProcessId=$procId" -ErrorAction Stop
      $cmd = [string]$w.CommandLine
      $exe = [string]$w.ExecutablePath
    } catch { }
    $isDev = ($DEV_NAMES -contains $name) -or ($cmd -match $DEV_LINE) -or ($DEV_PORTS -contains [int]$l.LocalPort)
    if (-not $isDev) { continue }
    $key = [int]$l.LocalPort
    if ($found.ContainsKey($key)) { continue }
    $found[$key] = [pscustomobject]@{
      port = $key
      name = $name
      cmd = $cmd
      exe = $exe
      project = (ProjectOf $cmd $exe)
      up = $true
    }
  }
  return @($found.Values | Sort-Object port)
}

function Title($entry) {
  $folder = if ($entry.project) { Split-Path -Leaf $entry.project } else { '' }
  if ($folder) { "$folder $($entry.port)" } else { "$($entry.name) $($entry.port)" }
}

# The name a project's servers go by: its folder, else the program.
function NameOf($entry) {
  if ($entry.project) { Split-Path -Leaf $entry.project } else { $entry.name }
}

# A widget id from words: a-z 0-9 _ -, at most 40.
function Slug($text, $fallback) {
  $slug = ($text.ToLower() -replace '[^a-z0-9_-]', '-') -replace '-{2,}', '-' -replace '^-|-$', ''
  if (-not $slug) { $slug = $fallback }
  if ($slug.Length -gt 24) { $slug = $slug.Substring(0, 24).TrimEnd('-') }
  return $slug
}

function Tell($entry, $seconds, $nudge) {
  $widget = @{
    id = "devserver-$($entry.port)"
    icon = 'server'
    color = $(if ($entry.up) { '#34d27b' } else { '#8e8e93' })
    label = (Title $entry)
    value = $(if ($entry.up) { '运行中' } else { '已停止' })
    detail = @{ sizes = @('s', 'm'); extra = $(if ($entry.cmd) { $entry.cmd } else { $entry.name }) }
    ttl = $seconds + 120
  }
  if ($nudge) { $widget.nudge = $nudge }
  Send $widget
}

# Several servers of one project in one widget (they would take a row each):
# the project's name, its ports as the value, and a row a server.
function TellGroup($entries, $seconds, $nudge) {
  $first = $entries[0]
  $name = NameOf $first
  $ports = @($entries | Sort-Object port | ForEach-Object { $_.port })
  $down = @($entries | Where-Object { -not $_.up })
  $value = if ($ports.Count -le 3) { $ports -join ' · ' } else { "$($ports.Count) 个服务" }
  if ($ports.Count -le 3 -and $down.Count) { $value += '（' + (($down | ForEach-Object { $_.port }) -join ',') + ' 停）' }
  if ($value.Length -gt 24) { $value = "$($ports.Count) 个服务" }
  $rows = @($entries | Sort-Object port | ForEach-Object {
    @{
      kind = 'port'
      name = "$($_.port)"
      text = "$($_.name) · $(if ($_.up) { '运行中' } else { '已停止' })"
    }
  })
  $widget = @{
    id = "devserver-$(Slug $name "group")-$($ports[0])"
    icon = 'server'
    color = $(if ($down.Count -eq 0) { '#34d27b' } elseif ($down.Count -eq $entries.Count) { '#8e8e93' } else { '#ffb340' })
    label = $name
    value = $value
    detail = @{ sizes = @('s', 'm', 'l'); extra = "$($entries.Count) 个服务"; rows = $rows }
    ttl = $seconds + 120
  }
  if ($nudge) { $widget.nudge = $nudge }
  Send $widget
}

# Start one kept as stopped: its command again, in its project, hidden.
function StartServer($entry) {
  if (-not $entry.cmd) { return $false }
  try {
    $shell = if (Get-Command pwsh -ErrorAction SilentlyContinue) { 'pwsh' } else { 'powershell' }
    $dir = if ($entry.project) { $entry.project } else { Split-Path -Parent $entry.exe }
    if (-not (Test-Path -LiteralPath $dir)) { $dir = $PSScriptRoot }
    $line = "Set-Location -LiteralPath '$($dir -replace "'", "''")'; $($entry.cmd)"
    # Encoded, so spaces and quotes in the command reach it whole (Start-Process
    # joins its arguments with spaces, which broke `node -e "..."`).
    $enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($line))
    Start-Process -FilePath $shell -ArgumentList @('-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', $enc) -WindowStyle Hidden | Out-Null
    return $true
  } catch { return $false }
}

$state = @(ReadState)
$was = @{}
foreach ($s in $state) { if ($s.port) { $was[[int]$s.port] = $s } }

# Asked to start some: theirs, kept in the state file, then done.
if ($Start.Count) {
  foreach ($p in $Start) {
    $n = 0
    if (-not [int]::TryParse($p, [ref]$n)) { continue }
    $entry = $was[$n]
    if ($entry -and (StartServer $entry)) { Write-Output "starting $($entry.name) on $n ($($entry.project))" }
    else { Write-Output "no command kept for $n" }
  }
  exit 0
}

$nudged = @{}
while ($true) {
  $now = @(Discover)
  $seen = @{}
  foreach ($e in $now) { $seen[$e.port] = $true }
  # Everything to show: what is up now, and what was kept (a day at most)
  # when it is not.
  $show = @($now)
  foreach ($p in @($was.Keys)) {
    if ($seen[$p]) { continue }
    $old = $was[$p]
    if (-not $old.seen) { continue }
    if ((New-TimeSpan -Start ([datetime]$old.seen) -End (Get-Date)).TotalHours -gt 24) { continue }
    $show += [pscustomobject]@{ port = $p; name = $old.name; cmd = $old.cmd; exe = $old.exe; project = $old.project; up = $false }
  }
  # One widget a project with several servers (a row a server); one alone as
  # it always was.
  foreach ($g in @($show | Group-Object { if ($_.project) { $_.project.ToLower() } else { "$($_.name):$($_.port)" } })) {
    $entries = @($g.Group)
    $nudge = $null
    foreach ($e in $entries) {
      if ($e.up) { $nudged.Remove($e.port) }
      elseif (-not $nudged[$e.port]) {
        $nudged[$e.port] = $true
        if (-not $nudge) { $nudge = "开发服务 $(NameOf $entries[0]) $($e.port) 停了" }
      }
    }
    if ($entries.Count -gt 1) { TellGroup $entries $Seconds $nudge }
    else { Tell $entries[0] $Seconds $nudge }
  }
  # What to remember: those up now (seen anew), and those kept as stopped.
  $keep = @()
  foreach ($e in $show) {
    if ($e.up) {
      $keep += [pscustomobject]@{ port = $e.port; name = $e.name; cmd = $e.cmd; exe = $e.exe; project = $e.project; seen = (Get-Date).ToString('o') }
    } elseif ($was[$e.port]) {
      $keep += $was[$e.port]
    }
  }
  WriteState $keep
  $was = @{}
  foreach ($k in $keep) { $was[[int]$k.port] = $k }
  if ($Once) { break }
  Start-Sleep -Seconds $Seconds
}
