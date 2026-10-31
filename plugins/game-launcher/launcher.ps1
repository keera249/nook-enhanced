# Game Launcher - a Nook plugin (live mode, PowerShell 5.1+, nothing to install).
#
# - Default view: clean icon-only grid (only the game's icon is shown).
# - Left-click any game icon: launches the game.
# - Right-click any game icon: opens the context menu for that game
#   (Change Icon, Reset Icon, Rename, Delete, Play).
# - Drop a .exe, .lnk, .url, or game folder onto the page anytime to add it.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Utf8 = New-Object System.Text.UTF8Encoding $false
$Out = [System.IO.StreamWriter]::new([Console]::OpenStandardOutput(), $Utf8)
$Out.AutoFlush = $true
$In = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), $Utf8)

$PluginDir = if ($env:NOOK_PLUGIN_DIR) { $env:NOOK_PLUGIN_DIR } else { $PSScriptRoot }
$DataDir = if ($env:NOOK_DATA_DIR) { $env:NOOK_DATA_DIR } else { Join-Path $PSScriptRoot '.data' }
$IconDir = Join-Path $DataDir 'icons'
$GamesFile = Join-Path $DataDir 'games.json'
New-Item -ItemType Directory -Force -Path $IconDir | Out-Null

Add-Type -Path (Join-Path $PluginDir 'NookIcon.cs') -ReferencedAssemblies System.Drawing, System.Windows.Forms

# Read placement from Nook's config.toml so mouse hit-testing knows where the panel sits.
function Read-PlacementFrac {
    try {
        $cfg = Join-Path $env:APPDATA 'Nook\config.toml'
        if (Test-Path -LiteralPath $cfg) {
            $txt = [System.IO.File]::ReadAllText($cfg)
            if ($txt -match '(?m)^\s*placement\s*=\s*"(left|center|right)"') {
                switch ($Matches[1]) {
                    'left' { return 0.0 }
                    'right' { return 1.0 }
                    default { return 0.5 }
                }
            }
        }
    }
    catch { }
    return 0.5
}

[NookBridge]::PlacementFrac = [float](Read-PlacementFrac)

# ---------------------------------------------------------------- state ----

$script:Games = [System.Collections.Generic.List[object]]::new()
$script:TileSize = 'large'
$script:PageWidth = 564
$script:PageHeight = 268
$script:InputN = 1
$script:RenameN = 1
$script:ShowAddBar = $false
$script:MenuGameId = $null
$script:Renaming = $false
$script:Status = $null

# ------------------------------------------------------------- plumbing ----

function Send($msg) {
    $Out.WriteLine((ConvertTo-Json -InputObject $msg -Depth 40 -Compress))
}

function Log([string]$text) {
    [Console]::Error.WriteLine("[game-launcher] $text")
}

function Set-Status([string]$text, [string]$color = 'dim') {
    $script:Status = @{ text = $text; color = $color }
}

function New-List { , ([System.Collections.Generic.List[object]]::new()) }

function Get-ShortHash([string]$s) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $bytes = $sha.ComputeHash($Utf8.GetBytes($s.ToLowerInvariant()))
    -join ($bytes[0..4] | ForEach-Object { $_.ToString('x2') })
}

# --------------------------------------------------------------- storage ----

function Load-Games {
    $script:Games.Clear()
    if (-not (Test-Path -LiteralPath $GamesFile)) { return }
    $migrated = $false
    try {
        $parsed = ConvertFrom-Json ([System.IO.File]::ReadAllText($GamesFile, $Utf8))
        foreach ($g in $parsed) {
            if (-not $g.id -or -not $g.target) { continue }
            $addedAt = [long]$g.addedAt
            if ($addedAt -le 0) {
                $png = Join-Path $IconDir ([string]$g.id + '.png')
                if (Test-Path -LiteralPath $png) {
                    $addedAt = [DateTimeOffset]::new((Get-Item -LiteralPath $png).CreationTimeUtc).ToUnixTimeSeconds()
                } else {
                    $addedAt = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
                }
                $migrated = $true
            }
            $script:Games.Add([pscustomobject]@{
                    id         = [string]$g.id
                    name       = [string]$g.name
                    kind       = if ($g.kind) { [string]$g.kind } else { 'exe' }
                    target     = [string]$g.target
                    args       = [string]$g.args
                    workdir    = [string]$g.workdir
                    iconSrc    = [string]$g.iconSrc
                    iconIndex  = [int]$g.iconIndex
                    customIcon = [bool]$g.customIcon
                    lastPlayed = [long]$g.lastPlayed
                    addedAt    = $addedAt
                    source     = if ($g.source) { [string]$g.source } else { 'manual' }
                    baseIcon   = [string]$g.baseIcon
                })
        }
    }
    catch {
        Log "could not read games.json: $($_.Exception.Message)"
    }
    if ($migrated) { Save-Games }
}

function Save-Games {
    $json = ConvertTo-Json -InputObject @($script:Games.ToArray()) -Depth 5
    $tmp = "$GamesFile.tmp"
    [System.IO.File]::WriteAllText($tmp, $json, $Utf8)
    [System.IO.File]::Copy($tmp, $GamesFile, $true)
    [System.IO.File]::Delete($tmp)
}

function Find-Game([string]$id) {
    if (-not $id) { return $null }
    foreach ($g in $script:Games) { if ($g.id -eq $id) { return $g } }
    return $null
}

# ----------------------------------------------------------------- icons ----

function Get-IconPath($g) {
    $png = Join-Path $IconDir ($g.id + '.png')
    if (Test-Path -LiteralPath $png) { return $png }
    return $null
}

function Update-Icon($g, [bool]$force = $false) {
    if (-not $g.iconSrc) { return }
    if ($g.customIcon -and -not $force) { return }
    $png = Join-Path $IconDir ($g.id + '.png')
    if (-not $force -and (Test-Path -LiteralPath $png)) { return }
    try {
        $r = [NookIcon]::Save($g.iconSrc, [int]$g.iconIndex, $png, 128)
        if ($r -ne 'ok' -and $g.iconSrc -ne $g.target -and (Test-Path -LiteralPath $g.target -PathType Leaf)) {
            $r = [NookIcon]::Save($g.target, 0, $png, 128)
        }
        if ($r -ne 'ok') { Log "icon for '$($g.name)': $r" }
    }
    catch {
        Log "icon for '$($g.name)': $($_.Exception.Message)"
    }
}

function Set-CustomIcon($g, [string]$filePath) {
    if (-not $g -or -not $filePath) { return $false }
    $p = $filePath.Trim().Trim('"', "'").Trim()
    if (-not (Test-Path -LiteralPath $p -PathType Leaf)) {
        Set-Status "Icon file not found: $p" 'red'
        return $false
    }
    $png = Join-Path $IconDir ($g.id + '.png')
    $r = [NookIcon]::Save($p, 0, $png, 128)
    if ($r -eq 'ok') {
        $g.iconSrc = $p
        $g.iconIndex = 0
        $g.customIcon = $true
        Save-Games
        Set-Status "Updated icon for $($g.name)" 'green'
        return $true
    }
    else {
        Set-Status "Could not load icon: $r" 'red'
        return $false
    }
}

# --------------------------------------------------------- name detection ----

$GenericNames = @(
    'game', 'games', 'launcher', 'game launcher', 'unity', 'unityplayer', 'unity player',
    'unreal', 'unreal engine', 'ue4', 'ue5', 'ue4game', 'unrealgame', 'shipping', 'bootstrap',
    'bootstrapper', 'application', 'app', 'main', 'client', 'start', 'setup', 'release',
    'default', 'project', 'godot', 'godot engine', 'electron', 'nw.js', 'node.js', 'windows',
    'microsoft windows operating system', 'win64', 'win32', 'x64', 'x86', 'bin', 'binaries',
    'my game', 'mygame', 'product', 'productname', 'exe'
)
$GenericDirs = '^(?i)(bin|bin32|bin64|binaries|win64|win32|x64|x86|game|games|retail|release|shipping|common|steamapps|steamlibrary|windowsnoeditor|windows|exe|launcher|app|client|program files.*|epic games|gog games|xboxgames|content)$'

function Clean-Name([string]$s) {
    if (-not $s) { return '' }
    $s = $s -replace '[\u00AE\u2122\u00A9]', ''
    $s = $s -replace '(?i)[-_ .]?(win64|win32|x64|x86|shipping|dx11|dx12|vulkan|retail)(?=$|[-_ .])', ''
    $s = $s -replace '_+', ' '
    ($s -replace '\s+', ' ').Trim(' ', '-', '.')
}

function Test-Generic([string]$s) {
    if (-not $s) { return $true }
    $k = ($s.ToLowerInvariant() -replace '[\u00AE\u2122\u00A9]', '').Trim()
    return ($k.Length -lt 2) -or ($GenericNames -contains $k) -or ($k -like '*operating system*')
}

function Get-ExeName([System.IO.FileInfo]$file) {
    try {
        $vi = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($file.FullName)
        foreach ($c in @([string]$vi.ProductName, [string]$vi.FileDescription)) {
            $n = (($c -replace '[\u00AE\u2122\u00A9]', '') -replace '\s+', ' ').Trim()
            if (-not (Test-Generic $n)) { return $n }
        }
    }
    catch { }
    $d = $file.Directory
    for ($i = 0; $i -lt 4 -and $d -and $d.Parent; $i++) {
        $n = Clean-Name $d.Name
        if ($d.Name -notmatch $GenericDirs -and -not (Test-Generic $n)) { return $n }
        $d = $d.Parent
    }
    $n = Clean-Name $file.BaseName
    if ($n) { return $n }
    return $file.BaseName
}

# ------------------------------------------------- pick the exe in a folder ----

$BadExe = '(?i)(unins|uninstall|setup|install|redist|vc_?redist|dxsetup|dxwebsetup|directx|crash|report|helper|cefsharp|cefprocess|webhelper|subprocess|updater|update|patch|easyanticheat|^eac|battleye|^be_|dotnet|prereq|overlay|steamworks|^7z|python|notification|wdapp|quicksfv|benchmark|dedicated|editor|cleanup|elevate|activation|register|unitycrashhandler|crashpad|vulkaninfo|dxdiag|oalinst|physx|touchup|repair|verify)'
$BadDir = '(?i)\\(_commonredist|commonredist|redist|redistributables?|__installer|installer|_installer|directx|dotnet|vcredist|prerequisites?|support|easyanticheat|battleye|engine\\binaries|tools|crashreport\w*|mono|jre|java|python|cef|webview\w*|\$pluginsdir)(\\|$)'

function Find-GameExe([string]$dir) {
    $dir = $dir.TrimEnd('\')
    $folderKey = ((Split-Path $dir -Leaf).ToLowerInvariant() -replace '[^a-z0-9]', '')
    $best = $null
    $bestScore = [double]::MinValue
    $exes = Get-ChildItem -LiteralPath $dir -Filter *.exe -File -Recurse -Depth 3 -ErrorAction SilentlyContinue
    foreach ($f in $exes) {
        $rel = $f.FullName.Substring($dir.Length).TrimStart('\')
        $relDir = Split-Path $rel -Parent
        $depth = if ($relDir) { $relDir.Split('\').Count } else { 0 }
        if ($f.BaseName -match $BadExe) { continue }
        if ($relDir -and ('\' + $relDir) -match $BadDir) { continue }

        $score = 0.0
        $k = ((Clean-Name $f.BaseName).ToLowerInvariant() -replace '[^a-z0-9]', '')
        if ($k -and $folderKey) {
            if ($k -eq $folderKey) { $score += 100 }
            elseif ($folderKey.Contains($k) -or $k.Contains($folderKey)) { $score += 60 }
        }
        try {
            $vi = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($f.FullName)
            $pk = (([string]$vi.ProductName).ToLowerInvariant() -replace '[^a-z0-9]', '')
            if ($pk -and $folderKey -and $pk -eq $folderKey) { $score += 50 }
        }
        catch { }
        $score += [Math]::Max(0, 30 - 15 * $depth)
        $score += [Math]::Min(40, $f.Length / 1MB / 5)
        if ($f.BaseName -match '(?i)launcher') { $score -= 5 }
        if ($score -gt $bestScore) { $bestScore = $score; $best = $f }
    }
    if ($best) { return $best.FullName }
    return $null
}

# ------------------------------------------------------------ adding games ----

function Split-IconLocation([string]$loc) {
    if (-not $loc) { return @('', 0) }
    $i = $loc.LastIndexOf(',')
    if ($i -lt 0) { return @($loc, 0) }
    $idx = 0
    [void][int]::TryParse($loc.Substring($i + 1).Trim(), [ref]$idx)
    return @($loc.Substring(0, $i).Trim(), $idx)
}

function Add-Game([string]$raw) {
    $p = $raw.Trim().Trim('"', "'").Trim()
    if (-not $p) { return $null }
    $p = [Environment]::ExpandEnvironmentVariables($p)
    if (-not (Test-Path -LiteralPath $p)) { Set-Status "Not found: $p" 'red'; return $null }

    $item = Get-Item -LiteralPath $p -Force
    if ($item.PSIsContainer) {
        $exe = Find-GameExe $item.FullName
        if (-not $exe) { Set-Status "No game .exe found in '$($item.Name)'" 'red'; return $null }
        Log "folder '$($item.FullName)' -> $exe"
        $item = Get-Item -LiteralPath $exe -Force
    }

    $g = [pscustomobject]@{
        id = ''; name = ''; kind = 'exe'; target = ''; args = ''; workdir = ''
        iconSrc = ''; iconIndex = 0; customIcon = $false; lastPlayed = 0
        addedAt = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
        source = 'manual'; baseIcon = ''
    }
    $ext = $item.Extension.ToLowerInvariant()

    if ($ext -eq '.lnk') {
        $shell = New-Object -ComObject WScript.Shell
        $lnk = $shell.CreateShortcut($item.FullName)
        $tp = [string]$lnk.TargetPath
        $g.name = $item.BaseName
        if ($tp -and $tp -match '(?i)\.exe$' -and (Test-Path -LiteralPath $tp -PathType Leaf)) {
            $g.target = $tp
            $g.args = [string]$lnk.Arguments
            $g.workdir = if ($lnk.WorkingDirectory) { [string]$lnk.WorkingDirectory } else { Split-Path $tp -Parent }
        }
        else {
            $g.kind = 'shell'
            $g.target = $item.FullName
        }
        $loc = Split-IconLocation ([Environment]::ExpandEnvironmentVariables([string]$lnk.IconLocation))
        if ($loc[0] -and (Test-Path -LiteralPath $loc[0] -PathType Leaf)) {
            $g.iconSrc = $loc[0]; $g.iconIndex = $loc[1]
        }
        elseif ($tp -and (Test-Path -LiteralPath $tp -PathType Leaf)) {
            $g.iconSrc = $tp
        }
        else {
            $g.iconSrc = $item.FullName
        }
        if ($g.kind -eq 'exe' -and (Test-Generic $g.name)) { $g.name = Get-ExeName (Get-Item -LiteralPath $tp) }
    }
    elseif ($ext -eq '.url') {
        $url = ''; $iconFile = ''; $iconIndex = 0
        foreach ($line in [System.IO.File]::ReadAllLines($item.FullName)) {
            if ($line -match '^\s*URL\s*=\s*(.+)$') { $url = $Matches[1].Trim() }
            elseif ($line -match '^\s*IconFile\s*=\s*(.+)$') { $iconFile = [Environment]::ExpandEnvironmentVariables($Matches[1].Trim()) }
            elseif ($line -match '^\s*IconIndex\s*=\s*(-?\d+)') { $iconIndex = [int]$Matches[1] }
        }
        $g.kind = 'url'
        $g.name = $item.BaseName
        $g.target = if ($url) { $url } else { $item.FullName }
        if ($iconFile -and (Test-Path -LiteralPath $iconFile -PathType Leaf)) {
            $g.iconSrc = $iconFile; $g.iconIndex = $iconIndex
        }
        else {
            $g.iconSrc = $item.FullName
        }
    }
    else {
        $g.kind = if ($ext -eq '.exe') { 'exe' } else { 'shell' }
        $g.target = $item.FullName
        $g.workdir = $item.DirectoryName
        $g.iconSrc = $item.FullName
        $g.name = if ($ext -eq '.exe') { Get-ExeName $item } else { Clean-Name $item.BaseName }
    }

    if (-not $g.name) { $g.name = $item.BaseName }
    $g.id = Get-ShortHash ($g.target + '|' + $g.args)
    if (Find-Game $g.id) { Set-Status "'$($g.name)' is already in the list" 'orange'; return $null }

    $script:Games.Add($g)
    Update-Icon $g $true
    Save-Games
    return $g
}

function Add-Paths($paths) {
    # If the right-click menu is open and the user drops an image (.png/.jpg/.ico), set it as that game's icon!
    $menuGame = Find-Game $script:MenuGameId
    if ($menuGame -and $paths.Count -eq 1) {
        $p = ([string]$paths[0]).Trim()
        if ($p -match '(?i)\.(png|jpg|jpeg|bmp|ico)$') {
            [void](Set-CustomIcon $menuGame $p)
            return
        }
    }

    $added = New-List
    foreach ($p in $paths) {
        try {
            $g = Add-Game ([string]$p)
            if ($g) { $added.Add($g.name) }
        }
        catch {
            Set-Status "Could not add '$p': $($_.Exception.Message)" 'red'
            Log "add '$p': $($_.Exception.Message)"
        }
    }
    if ($added.Count -gt 0) { Set-Status ("Added: " + ($added -join ', ')) 'green' }
}

# ------------------------------------------------- auto-detect installed games ----
# Steam, Epic Games and GOG Galaxy libraries are read from their own manifests.
# Steam / Epic games launch through their launcher (steam://rungameid/...), so
# overlay, cloud saves and DRM work exactly like pressing Play in Steam.
# Games you delete from the grid are remembered in ignored.json and not re-added.

$IgnoredFile = Join-Path $DataDir 'ignored.json'
$script:Ignored = [System.Collections.Generic.HashSet[string]]::new()

function Load-Ignored {
    $script:Ignored.Clear()
    if (-not (Test-Path -LiteralPath $IgnoredFile)) { return }
    try {
        foreach ($i in (ConvertFrom-Json ([System.IO.File]::ReadAllText($IgnoredFile, $Utf8)))) {
            if ($i) { [void]$script:Ignored.Add([string]$i) }
        }
    }
    catch { Log "could not read ignored.json: $($_.Exception.Message)" }
}

function Save-Ignored {
    $json = ConvertTo-Json -InputObject @($script:Ignored | ForEach-Object { $_ }) -Depth 2
    [System.IO.File]::WriteAllText($IgnoredFile, $json, $Utf8)
}

function Strip-Marks([string]$s) {
    if (-not $s) { return '' }
    (($s -replace '[\u00AE\u2122\u00A9]', '') -replace '\s+', ' ').Trim()
}

function Get-SteamLibraries {
    $libs = New-List
    $steam = $null
    foreach ($k in 'HKCU:\Software\Valve\Steam', 'HKLM:\SOFTWARE\WOW6432Node\Valve\Steam', 'HKLM:\SOFTWARE\Valve\Steam') {
        try {
            $p = Get-ItemProperty -Path $k -ErrorAction Stop
            $v = if ($p.SteamPath) { $p.SteamPath } else { $p.InstallPath }
            if ($v -and (Test-Path -LiteralPath $v)) { $steam = ($v -replace '/', '\'); break }
        }
        catch { }
    }
    if (-not $steam) { return @{ root = $null; libs = $libs } }
    $libs.Add($steam)
    $vdf = Join-Path $steam 'steamapps\libraryfolders.vdf'
    if (Test-Path -LiteralPath $vdf) {
        $txt = [System.IO.File]::ReadAllText($vdf, $Utf8)
        foreach ($m in [regex]::Matches($txt, '"path"\s+"([^"]+)"')) {
            $p = $m.Groups[1].Value -replace '\\\\', '\'
            if ((Test-Path -LiteralPath $p) -and -not ($libs | Where-Object { $_ -ieq $p })) { $libs.Add($p) }
        }
    }
    return @{ root = $steam; libs = $libs }
}

# Steam tools that are not games.
$SteamSkipIds = @('228980', '250820', '1070560', '1391110', '1628350', '1493710', '2180100', '1826330', '961940', '858280', '1580130', '1887720')
$SteamSkipName = '(?i)(redistributable|steamworks|proton|steam linux runtime|steamvr|dedicated server|soundtrack|\bsdk\b)'

function Find-SteamArtIcon([string]$root, [string]$appId) {
    if (-not $root) { return $null }
    $dir = Join-Path $root "appcache\librarycache\$appId"
    if (Test-Path -LiteralPath $dir) {
        # The small square icon is stored under its content hash (40 hex chars).
        $ico = Get-ChildItem -LiteralPath $dir -File -Recurse -ErrorAction SilentlyContinue |
        Where-Object { $_.BaseName -match '^[0-9a-f]{40}$' } | Select-Object -First 1
        if ($ico) { return $ico.FullName }
        foreach ($n in 'library_600x900.jpg', 'header.jpg') {
            $f = Get-ChildItem -LiteralPath $dir -Filter $n -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($f) { return $f.FullName }
        }
    }
    $old = Join-Path $root "appcache\librarycache\${appId}_icon.jpg"
    if (Test-Path -LiteralPath $old) { return $old }
    return $null
}

function Get-SteamGames {
    $found = New-List
    $info = Get-SteamLibraries
    foreach ($lib in $info.libs) {
        $apps = Join-Path $lib 'steamapps'
        foreach ($acf in (Get-ChildItem -LiteralPath $apps -Filter 'appmanifest_*.acf' -File -ErrorAction SilentlyContinue)) {
            try {
                $t = [System.IO.File]::ReadAllText($acf.FullName, $Utf8)
                $appId = [regex]::Match($t, '"appid"\s+"(\d+)"').Groups[1].Value
                $name = Strip-Marks ([regex]::Match($t, '"name"\s+"([^"]*)"').Groups[1].Value)
                $dir = [regex]::Match($t, '"installdir"\s+"([^"]*)"').Groups[1].Value
                $flags = 0
                [void][int]::TryParse([regex]::Match($t, '"StateFlags"\s+"(\d+)"').Groups[1].Value, [ref]$flags)
                if (-not $appId -or -not $name) { continue }
                if ($SteamSkipIds -contains $appId -or $name -match $SteamSkipName) { continue }
                if (($flags -band 4) -eq 0) { continue }   # not fully installed yet
                $installDir = Join-Path $apps "common\$dir"
                if (-not (Test-Path -LiteralPath $installDir)) { continue }
                $found.Add([pscustomobject]@{
                        key = "steam:$appId"; name = $name; kind = 'url'
                        target = "steam://rungameid/$appId"; installDir = $installDir
                        exe = $null; art = (Find-SteamArtIcon $info.root $appId); source = 'steam'
                    })
            }
            catch { Log "steam manifest $($acf.Name): $($_.Exception.Message)" }
        }
    }
    return $found
}

function Get-EpicGames {
    $found = New-List
    $dir = Join-Path $env:ProgramData 'Epic\EpicGamesLauncher\Data\Manifests'
    foreach ($f in (Get-ChildItem -LiteralPath $dir -Filter *.item -File -ErrorAction SilentlyContinue)) {
        try {
            $j = ConvertFrom-Json ([System.IO.File]::ReadAllText($f.FullName, $Utf8))
            $cats = @($j.AppCategories)
            if ($cats -contains 'hidden' -or -not ($cats -contains 'games')) { continue }
            if ($j.bIsIncompleteInstall -or -not $j.InstallLocation -or -not (Test-Path -LiteralPath $j.InstallLocation)) { continue }
            if ($j.MainGameAppName -and $j.MainGameAppName -ne $j.AppName) { continue }   # DLC
            $exe = $null
            if ($j.LaunchExecutable) {
                $e = Join-Path $j.InstallLocation $j.LaunchExecutable
                if (Test-Path -LiteralPath $e -PathType Leaf) { $exe = $e }
            }
            $uri = "com.epicgames.launcher://apps/$($j.CatalogNamespace)%3A$($j.CatalogItemId)%3A$($j.AppName)?action=launch&silent=true"
            $found.Add([pscustomobject]@{
                    key = "epic:$($j.AppName)"; name = (Strip-Marks $j.DisplayName); kind = 'url'
                    target = $uri; installDir = [string]$j.InstallLocation
                    exe = $exe; art = $null; source = 'epic'
                })
        }
        catch { Log "epic manifest $($f.Name): $($_.Exception.Message)" }
    }
    return $found
}

function Get-GogGames {
    $found = New-List
    foreach ($root in 'HKLM:\SOFTWARE\WOW6432Node\GOG.com\Games', 'HKLM:\SOFTWARE\GOG.com\Games') {
        foreach ($k in (Get-ChildItem -Path $root -ErrorAction SilentlyContinue)) {
            try {
                $p = Get-ItemProperty -Path $k.PSPath
                if ($p.dependsOn) { continue }   # DLC
                $exe = [string]$p.exe
                if (-not $exe -or -not (Test-Path -LiteralPath $exe -PathType Leaf)) { continue }
                $found.Add([pscustomobject]@{
                        key = "gog:$($k.PSChildName)"; name = (Strip-Marks $p.gameName); kind = 'exe'
                        target = $exe; installDir = [string]$p.path; exe = $exe; art = $null; source = 'gog'
                        args = [string]$p.launchParam
                        workdir = if ($p.workingDir) { [string]$p.workingDir } else { Split-Path $exe -Parent }
                    })
            }
            catch { }
        }
    }
    return $found
}

function Sync-InstalledGames([bool]$announce = $false) {
    $detected = New-List
    foreach ($fn in 'Get-SteamGames', 'Get-EpicGames', 'Get-GogGames') {
        try { foreach ($d in (& $fn)) { $detected.Add($d) } } catch { Log "$fn : $($_.Exception.Message)" }
    }
    $added = New-List
    $removed = 0
    $liveIds = [System.Collections.Generic.HashSet[string]]::new()

    foreach ($d in $detected) {
        $id = Get-ShortHash $d.key
        [void]$liveIds.Add($id)
        if ($script:Ignored.Contains($id) -or (Find-Game $id)) { continue }
        # Skip games already added by hand from the same install folder.
        $dup = $false
        if ($d.installDir) {
            $root = $d.installDir.TrimEnd('\') + '\'
            foreach ($g in $script:Games) {
                if ($g.source -eq 'manual' -and $g.target -and $g.target.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) { $dup = $true; break }
            }
        }
        if ($dup) { continue }

        # Icon: the game's own .exe gives the sharpest icon; Steam's art cache is the fallback.
        $exe = $d.exe
        if (-not $exe -and $d.installDir) { try { $exe = Find-GameExe $d.installDir } catch { } }
        $iconSrc = if ($exe) { $exe } elseif ($d.art) { $d.art } else { '' }

        $g = [pscustomobject]@{
            id = $id; name = $d.name; kind = $d.kind; target = $d.target
            args = [string]$d.args; workdir = [string]$d.workdir
            iconSrc = $iconSrc; iconIndex = 0; customIcon = $false; lastPlayed = 0
            addedAt = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
            source = $d.source; baseIcon = $iconSrc
        }
        $script:Games.Add($g)
        Update-Icon $g $true
        if (-not (Get-IconPath $g) -and $d.art -and $d.art -ne $iconSrc) {
            $g.iconSrc = $d.art; $g.baseIcon = $d.art
            Update-Icon $g $true
        }
        $added.Add($g.name)
        Log "detected $($d.source) game '$($g.name)'"
    }

    # Drop auto-detected games that were uninstalled.
    foreach ($g in @($script:Games.ToArray())) {
        if ($g.source -in 'steam', 'epic', 'gog' -and -not $liveIds.Contains($g.id)) {
            [void]$script:Games.Remove($g)
            $png = Join-Path $IconDir ($g.id + '.png')
            if (Test-Path -LiteralPath $png) { Remove-Item -LiteralPath $png -Force -ErrorAction SilentlyContinue }
            $removed++
        }
    }

    if ($added.Count -gt 0 -or $removed -gt 0) { Save-Games }
    if ($announce -or $added.Count -gt 0) {
        if ($added.Count -gt 0) { Set-Status ("Found: " + ($added -join ', ')) 'green' }
        elseif ($announce) { Set-Status "Scanned Steam, Epic and GOG: no new games ($($detected.Count) installed)" 'dim' }
    }
    return ($added.Count -gt 0 -or $removed -gt 0)
}

# -------------------------------------------------------------- launching ----

function Start-Game($g) {
    if (-not $g) { return }
    try {
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = $g.target
        $psi.UseShellExecute = $true
        if ($g.kind -eq 'exe') {
            if ($g.args) { $psi.Arguments = $g.args }
            $wd = $g.workdir
            if (-not $wd -or -not (Test-Path -LiteralPath $wd)) { $wd = Split-Path $g.target -Parent }
            $psi.WorkingDirectory = $wd
        }
        [void][System.Diagnostics.Process]::Start($psi)
        $g.lastPlayed = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
        Save-Games
        Set-Status "Launched $($g.name)" 'green'
        Log "launched '$($g.name)' -> $($g.target)"
    }
    catch {
        Set-Status "Could not launch $($g.name): $($_.Exception.Message)" 'red'
        Log "launch '$($g.name)': $($_.Exception.Message)"
    }
}

function Get-LastGame {
    $last = $null
    foreach ($g in $script:Games) {
        if (-not $last -or [long]$g.lastPlayed -gt [long]$last.lastPlayed) { $last = $g }
    }
    return $last
}

function Get-RecentGames {
    $indexed = for ($i = 0; $i -lt $script:Games.Count; $i++) {
        $g = $script:Games[$i]
        $ts = [Math]::Max([long]$g.lastPlayed, [long]$g.addedAt)
        [pscustomobject]@{ idx = $i; ts = $ts; g = $g }
    }
    @($indexed | Sort-Object -Property @{Expression = 'ts'; Descending = $true }, @{Expression = 'idx'; Descending = $true } | ForEach-Object { $_.g })
}

# ---------------------------------------------------------------- drawing ----

function Text([string]$t, [string]$size = 'body', [string]$color = $null) {
    $n = @{ type = 'text'; text = $t; size = $size }
    if ($color) { $n.color = $color }
    $n
}

function GameImage($g, [int]$px, [string]$tooltip = $null, [string]$clickId = $null) {
    $png = if ($g) { Get-IconPath $g } else { $null }
    if ($png) {
        $n = @{ type = 'image'; path = $png; width = $px; height = $px; rounding = [Math]::Round($px / 4.2) }
    }
    else {
        $n = @{ type = 'icon'; icon = 'game-controller'; size = [Math]::Round($px * 0.85); color = 'accent' }
    }
    if ($tooltip) { $n.tooltip = $tooltip }
    if ($clickId) { $n.id = $clickId }
    $n
}

function Draw-Tile {
    $recent = @(Get-RecentGames)
    if ($recent.Count -eq 0) {
        if ($script:TileSize -eq 'small') {
            Send @{ type = 'tile'; view = @{ type = 'column'; align = 'center'; justify = 'center'; children = @(
                        @{ type = 'icon'; icon = 'game-controller'; size = 26; color = 'accent'; tooltip = 'Game Launcher: click to open' }) 
                } 
            }
        }
        else {
            Send @{ type = 'tile'; view = @{ type = 'row'; align = 'center'; gap = 8; children = @(
                        @{ type = 'icon'; icon = 'game-controller'; size = 24; color = 'accent' },
                        @{ type = 'column'; gap = 0; children = @((Text 'Games' 'title'), (Text 'Click to open' 'small' 'dim')) }) 
                } 
            }
        }
        return
    }

    switch ($script:TileSize) {
        'small' {
            $first = $recent[0]
            Send @{ type = 'tile'; view = @{ type = 'column'; align = 'center'; justify = 'center'; children = @(
                        (GameImage $first 30 $first.name "launch:$($first.id)")) 
                } 
            }
        }
        'large' {
            # Show the 6 most-recently-played games as a clean 3x2 icon grid right on the tile.
            # Clicking any icon launches that game immediately; clicking empty space opens the full folder.
            $cols = 3
            $maxIcons = 6
            $shown = @($recent | Select-Object -First $maxIcons)
            $rows = New-List
            for ($r = 0; $r -lt $shown.Count; $r += $cols) {
                $rowKids = New-List
                for ($c = 0; $c -lt $cols -and ($r + $c) -lt $shown.Count; $c++) {
                    $g = $shown[$r + $c]
                    $rowKids.Add((GameImage $g 30 $g.name "launch:$($g.id)"))
                }
                $rows.Add(@{ type = 'row'; align = 'center'; gap = 7; children = $rowKids })
            }
            Send @{ type = 'tile'; view = @{ type = 'column'; gap = 8; children = $rows } }
        }
        default {
            # Wide (2x1): show the top 3 most-recently-played game icons in a row.
            $shown = @($recent | Select-Object -First 3)
            $rowKids = New-List
            foreach ($g in $shown) {
                $rowKids.Add((GameImage $g 28 $g.name "launch:$($g.id)"))
            }
            Send @{ type = 'tile'; view = @{ type = 'row'; align = 'center'; gap = 8; children = $rowKids } }
        }
    }
}

function Draw-Page {
    $kids = New-List
    $topOffset = 0.0

    # Optional Add-by-path bar (toggled with the + button in the header, or shown when list is empty).
    if ($script:ShowAddBar -or $script:Games.Count -eq 0) {
        $kids.Add(@{ type = 'input'; id = "path:$($script:InputN)"; hint = 'Paste a game .exe, shortcut or game folder path and press Enter (or drop files here)' })
        $topOffset += 36.0
    }

    if ($script:Status) {
        $kids.Add((Text $script:Status.text 'small' $script:Status.color))
        $topOffset += 22.0
    }

    # Right-click context bar for the selected game (only appears on right-click!).
    $sel = Find-Game $script:MenuGameId
    if ($sel) {
        $menuKids = New-List
        $menuKids.Add(@{
                type = 'row'; align = 'center'; gap = 8; children = @(
                    (GameImage $sel 28 $sel.target),
                    (Text $sel.name 'title'),
                    @{ type = 'spacer' },
                    @{ type = 'button'; id = "launch:$($sel.id)"; label = 'Play'; icon = 'play'; primary = $true },
                    @{ type = 'button'; id = "change_icon:$($sel.id)"; label = 'Change Icon'; icon = 'image'; tooltip = 'Pick a custom icon image (.png, .jpg, .ico, .exe)' },
                    @{ type = 'button'; id = "reset_icon:$($sel.id)"; icon = 'arrow-clockwise'; tooltip = 'Restore original .exe icon' },
                    @{ type = 'button'; id = "toggle_rename:$($sel.id)"; icon = 'pencil-simple'; tooltip = 'Rename game' },
                    @{ type = 'button'; id = "remove:$($sel.id)"; icon = 'trash'; tooltip = 'Delete game' },
                    @{ type = 'button'; id = 'close_menu'; icon = 'x'; tooltip = 'Close menu' }
                )
            })
        if ($script:Renaming) {
            $menuKids.Add(@{ type = 'input'; id = "rename:$($sel.id):$($script:RenameN)"; value = $sel.name; hint = 'Type new name and press Enter' })
            $topOffset += 34.0
        }
        $kids.Add(@{ type = 'card'; padding = 8; gap = 6; children = $menuKids })
        $topOffset += 50.0
    }

    $cols = 7
    [NookBridge]::Columns = $cols
    [NookBridge]::GameCount = $script:Games.Count
    [NookBridge]::GridTopOffset = [float]$topOffset

    if ($script:Games.Count -eq 0) {
        $kids.Add(@{ type = 'card'; children = @(
                    @{ type = 'row'; align = 'center'; gap = 10; children = @(
                            @{ type = 'icon'; icon = 'file-arrow-down'; size = 28; color = 'accent' },
                            @{ type = 'column'; gap = 2; children = @(
                                    (Text 'No games yet' 'title'),
                                    @{ type = 'text'; text = "Drag a game's .exe, its shortcut (.lnk / Steam .url) or the whole game folder onto this page."; size = 'small'; color = 'dim'; wrap = $true }) 
                            }) 
                    }) 
            })
    }
    else {
        $iconPx = [int][Math]::Max(36, [Math]::Round((($script:PageWidth - 10.0 * ($cols - 1)) / $cols) - 20.0))
        $cards = New-List
        foreach ($g in (Get-RecentGames)) {
            $tip = "$($g.name)`nLeft-click: Play  |  Right-click: Change Icon / Delete"
            $cards.Add(@{
                    type     = 'card'
                    padding  = 10
                    tooltip  = $tip
                    children = @( (GameImage $g $iconPx $tip "page_game:$($g.id)") )
                })
        }
        $kids.Add(@{ type = 'grid'; columns = $cols; gap = 10; children = $cards })
    }

    Send @{
        type    = 'page'
        actions = @(
            @{ id = 'scan'; icon = 'magnifying-glass'; tooltip = 'Scan Steam, Epic and GOG for installed games' },
            @{ id = 'toggle_add'; icon = 'plus'; tooltip = 'Add game by path' },
            @{ id = 'restore_hidden'; icon = 'eye'; tooltip = 'Bring back detected games you removed' },
            @{ id = 'refresh'; icon = 'arrow-clockwise'; tooltip = 'Refresh icons' },
            @{ id = 'open_data'; icon = 'folder-open'; tooltip = 'Open data folder (games.json, icons)' })
        view    = @{ type = 'column'; gap = 8; scroll = $true; children = $kids }
    }
}

function Draw-All { Draw-Tile; Draw-Page }

# ---------------------------------------------------------------- events ----

function Handle-Event($msg) {
    $id = [string]$msg.id
    $kind = [string]$msg.kind
    if ($id -eq 'play_last') { Start-Game (Get-LastGame) }
    elseif ($id -eq 'toggle_add') {
        $script:ShowAddBar = -not $script:ShowAddBar
        $script:Status = $null
    }
    elseif ($id -eq 'close_menu') {
        $script:MenuGameId = $null
        $script:Renaming = $false
        $script:Status = $null
    }
    elseif ($id -like 'launch:*') {
        $g = Find-Game $id.Substring(7)
        $script:MenuGameId = $null
        $script:Renaming = $false
        Start-Game $g
    }
    elseif ($id -like 'page_game:*') {
        $gid = $id.Substring(10)
        $g = Find-Game $gid
        if (-not $g) { return }
        if ($kind -eq 'right_click') {
            if ($script:MenuGameId -eq $g.id) {
                $script:MenuGameId = $null
                $script:Renaming = $false
            }
            else {
                $script:MenuGameId = $g.id
                $script:Renaming = $false
                $script:Status = $null
            }
            Draw-Page
            return
        }
        else {
            $script:MenuGameId = $null
            $script:Renaming = $false
            Start-Game $g
        }
    }
    elseif ($id -like 'change_icon:*') {
        $g = Find-Game $id.Substring(12)
        if ($g) {
            $picked = [NookIcon]::PickIconFile("Choose icon for $($g.name)")
            if ($picked) {
                [void](Set-CustomIcon $g $picked)
            }
        }
    }
    elseif ($id -like 'reset_icon:*') {
        $g = Find-Game $id.Substring(11)
        if ($g) {
            $g.customIcon = $false
            if ($g.baseIcon -and (Test-Path -LiteralPath $g.baseIcon -PathType Leaf)) { $g.iconSrc = $g.baseIcon }
            elseif (Test-Path -LiteralPath $g.target -PathType Leaf) { $g.iconSrc = $g.target }
            $g.iconIndex = 0
            Update-Icon $g $true
            Save-Games
            Set-Status "Restored default icon for $($g.name)" 'green'
        }
    }
    elseif ($id -like 'toggle_rename:*') {
        $script:Renaming = -not $script:Renaming
        $script:RenameN++
    }
    elseif ($id -like 'rename:*' -and $kind -eq 'submit') {
        $parts = $id.Split(':')
        $g = Find-Game $parts[1]
        $new = ([string]$msg.value).Trim()
        if ($g -and $new) { $g.name = $new; Save-Games; Set-Status "Renamed to $new" 'green' }
        $script:Renaming = $false
        $script:RenameN++
    }
    elseif ($id -like 'remove:*') {
        $g = Find-Game $id.Substring(7)
        if ($g) {
            [void]$script:Games.Remove($g)
            if ($g.source -in 'steam', 'epic', 'gog') { [void]$script:Ignored.Add($g.id); Save-Ignored }
            $png = Join-Path $IconDir ($g.id + '.png')
            if (Test-Path -LiteralPath $png) { Remove-Item -LiteralPath $png -Force }
            Save-Games
            $script:MenuGameId = $null
            $script:Renaming = $false
            Set-Status "Removed $($g.name)" 'dim'
        }
    }
    elseif ($id -like 'path:*' -and $kind -eq 'submit') {
        $script:InputN++
        Add-Paths @([string]$msg.value)
        $script:ShowAddBar = $false
    }
    elseif ($id -eq 'refresh') {
        foreach ($g in $script:Games) { Update-Icon $g $true }
        Set-Status 'Icons refreshed' 'green'
    }
    elseif ($id -eq 'scan') {
        [void](Sync-InstalledGames $true)
    }
    elseif ($id -eq 'restore_hidden') {
        $n = $script:Ignored.Count
        $script:Ignored.Clear()
        Save-Ignored
        [void](Sync-InstalledGames $false)
        if ($n -gt 0) { Set-Status "Restored $n removed game(s)" 'green' } else { Set-Status 'Nothing to restore' 'dim' }
    }
    elseif ($id -eq 'open_data') {
        Send @{ type = 'open'; target = $DataDir }
        return
    }
    else { return }
    Draw-All
}

function Handle-Mouse($msg) {
    $idx = [int]$msg.index
    if ($idx -lt 0 -or $idx -ge $script:Games.Count) { return }
    $g = $script:Games[$idx]
    if ([string]$msg.button -eq 'right') {
        if ($script:MenuGameId -eq $g.id) {
            $script:MenuGameId = $null
            $script:Renaming = $false
        }
        else {
            $script:MenuGameId = $g.id
            $script:Renaming = $false
            $script:Status = $null
        }
        Draw-Page
    }
    elseif ([string]$msg.button -eq 'left') {
        $script:MenuGameId = $null
        $script:Renaming = $false
        Start-Game $g
        Draw-All
    }
}

function Handle($msg) {
    switch ([string]$msg.type) {
        'event' { Handle-Event $msg }
        '_mouse' { Handle-Mouse $msg }
        'files' { Add-Paths @($msg.paths); Draw-All }
        'layout' {
            if ($msg.target -eq 'tile' -and $msg.size -and $msg.size -ne $script:TileSize) {
                $script:TileSize = [string]$msg.size; Draw-Tile
            }
            elseif ($msg.target -eq 'page') {
                $changed = ($msg.width -and [Math]::Abs([double]$msg.width - $script:PageWidth) -gt 0.5)
                if ($msg.width) { $script:PageWidth = [double]$msg.width; [NookBridge]::BodyWidth = [float]$msg.width }
                if ($msg.height) { $script:PageHeight = [double]$msg.height; [NookBridge]::BodyHeight = [float]$msg.height }
                if ($changed) { Draw-Page }
            }
        }
        'page_open' {
            # Page opened
        }
        'page_close' {
            if ($script:Status -or $script:MenuGameId -or $script:Renaming -or $script:ShowAddBar) {
                $script:Status = $null
                $script:MenuGameId = $null
                $script:Renaming = $false
                $script:ShowAddBar = $false
                Draw-Page
            }
        }
        'shutdown' { return 'stop' }
    }
}

# ------------------------------------------------------------------ main ----

Load-Games
Load-Ignored
foreach ($g in $script:Games) { Update-Icon $g $false }
Draw-All
try { if (Sync-InstalledGames $false) { Draw-All } } catch { Log "scan: $($_.Exception.Message)" }

while ($true) {
    $line = $In.ReadLine()
    if ($null -eq $line) { break }
    if (-not $line.Trim()) { continue }
    try { $msg = ConvertFrom-Json $line } catch { continue }
    try {
        if ((Handle $msg) -eq 'stop') { break }
    }
    catch {
        Log "error: $($_.Exception.Message) at $($_.InvocationInfo.ScriptLineNumber)"
    }
}
