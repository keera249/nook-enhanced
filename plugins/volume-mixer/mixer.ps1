# Per-App Volume Mixer - a Nook plugin (live mode, PowerShell 5.1+).
# Controls Master Speaker volume and individual volume/mute for every open app.

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Utf8 = New-Object System.Text.UTF8Encoding $false
$Out = [System.IO.StreamWriter]::new([Console]::OpenStandardOutput(), $Utf8)
$Out.AutoFlush = $true
$In = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), $Utf8)

$PluginDir = if ($env:NOOK_PLUGIN_DIR) { $env:NOOK_PLUGIN_DIR } else { $PSScriptRoot }
$DataDir = if ($env:NOOK_DATA_DIR) { $env:NOOK_DATA_DIR } else { Join-Path $PSScriptRoot '.data' }
$IconDir = Join-Path $DataDir 'icons'
New-Item -ItemType Directory -Force -Path $IconDir | Out-Null

Add-Type -Path (Join-Path $PluginDir 'AudioMixer.cs') -ReferencedAssemblies System.Drawing

$script:TileSize = 'wide'
$script:Apps = @()
$script:MasterVol = 100
$script:MasterMuted = $false

function Send($msg) {
    $Out.WriteLine((ConvertTo-Json -InputObject $msg -Depth 40 -Compress))
}

function Log([string]$text) {
    [Console]::Error.WriteLine("[volume-mixer] $text")
}

function New-List { , ([System.Collections.Generic.List[object]]::new()) }

function Refresh-Audio {
    $muted = $false
    $script:MasterVol = [AudioMixer]::GetMasterVolume([ref]$muted)
    $script:MasterMuted = $muted
    $script:Apps = @([AudioMixer]::GetApps())
    foreach ($a in $script:Apps) {
        if ($a.ExePath) {
            $png = Join-Path $IconDir ($a.Key + '.png')
            [AudioMixer]::SaveExeIcon($a.ExePath, $png)
        }
    }
}

function Get-AppIconNode($a) {
    $png = Join-Path $IconDir ($a.Key + '.png')
    if (Test-Path -LiteralPath $png) {
        return @{ type = 'image'; path = $png; width = 22; height = 22; rounding = 5 }
    }
    $ico = if ($a.Key -eq 'system') { 'speaker-hifi' } else { 'speaker-high' }
    return @{ type = 'icon'; icon = $ico; size = 20; color = 'accent' }
}

function Draw-Tile {
    $mIcon = if ($script:MasterMuted -or $script:MasterVol -eq 0) { 'speaker-slash' } elseif ($script:MasterVol -lt 50) { 'speaker-low' } else { 'speaker-high' }
    $mColor = if ($script:MasterMuted) { 'red' } else { 'accent' }
    $volTxt = if ($script:MasterMuted) { 'Muted' } else { "$($script:MasterVol)%" }
    $appCount = $script:Apps.Count

    switch ($script:TileSize) {
        'small' {
            Send @{
                type = 'tile'
                view = @{
                    type = 'ring'
                    value = [Math]::Round($script:MasterVol / 100.0, 2)
                    label = "$($script:MasterVol)"
                    color = $mColor
                    tooltip = "Volume Mixer ($appCount apps)"
                }
            }
        }
        default {
            Send @{
                type = 'tile'
                view = @{
                    type = 'row'; align = 'center'; gap = 8
                    children = @(
                        @{ type = 'icon'; icon = 'faders'; size = 22; color = $mColor },
                        @{
                            type = 'column'; gap = 0
                            children = @(
                                @{ type = 'text'; text = "Mixer ($appCount)"; size = 'small'; color = 'dim' },
                                @{ type = 'text'; text = $volTxt; size = 'title' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{ type = 'button'; id = 'mute:master'; icon = $mIcon; tooltip = 'Mute / Unmute Master' }
                    )
                }
            }
        }
    }
}

function Draw-Page {
    $kids = New-List

    # Master Speaker Card
    $mIcon = if ($script:MasterMuted -or $script:MasterVol -eq 0) { 'speaker-slash' } else { 'speaker-high' }
    $mLabel = if ($script:MasterMuted) { 'Muted' } else { "$($script:MasterVol)%" }
    $mColor = if ($script:MasterMuted) { 'red' } else { 'accent' }

    $kids.Add(@{
        type = 'card'; padding = 10; gap = 6
        children = @(
            @{
                type = 'row'; align = 'center'; gap = 8
                children = @(
                    @{ type = 'icon'; icon = 'speaker-hifi'; size = 20; color = $mColor },
                    @{ type = 'text'; text = 'Master Volume'; size = 'body'; bold = $true },
                    @{ type = 'spacer' },
                    @{ type = 'text'; text = $mLabel; size = 'small'; color = 'dim'; mono = $true },
                    @{ type = 'button'; id = 'mute:master'; icon = $mIcon; tooltip = 'Toggle Master Mute' }
                )
            },
            @{
                type = 'slider'
                id = 'vol:master'
                value = [int]$script:MasterVol
                min = 0
                max = 100
                step = 1
            }
        )
    })

    # Per-App Sliders
    foreach ($a in $script:Apps) {
        $aIcon = if ($a.Muted -or $a.Volume -eq 0) { 'speaker-slash' } else { 'speaker-high' }
        $aLabel = if ($a.Muted) { 'Muted' } else { "$($a.Volume)%" }
        $statusDot = if ($a.Active) { ' (Playing)' } else { '' }

        $kids.Add(@{
            type = 'card'; padding = 8; gap = 4
            children = @(
                @{
                    type = 'row'; align = 'center'; gap = 8
                    children = @(
                        (Get-AppIconNode $a),
                        @{ type = 'text'; text = "$($a.Name)$statusDot"; size = 'body' },
                        @{ type = 'spacer' },
                        @{ type = 'text'; text = $aLabel; size = 'small'; color = $(if ($a.Muted) { 'red' } else { 'dim' }); mono = $true },
                        @{ type = 'button'; id = "mute:$($a.Key)"; icon = $aIcon; tooltip = "Mute / Unmute $($a.Name)" }
                    )
                },
                @{
                    type = 'slider'
                    id = "vol:$($a.Key)"
                    value = [int]$a.Volume
                    min = 0
                    max = 100
                    step = 1
                }
            )
        })
    }

    Send @{
        type = 'page'
        actions = @(
            @{ id = 'refresh'; icon = 'arrow-clockwise'; tooltip = 'Refresh open audio apps' }
        )
        view = @{ type = 'column'; gap = 8; scroll = $true; children = $kids }
    }
}

function Draw-All {
    Refresh-Audio
    Draw-Tile
    Draw-Page
}

function Handle-Event($msg) {
    $id = [string]$msg.id
    $kind = [string]$msg.kind

    if ($id -eq 'refresh') {
        Draw-All
        return
    }
    if ($id -eq 'mute:master') {
        [AudioMixer]::ToggleMasterMute()
        Draw-All
        return
    }
    if ($id -like 'mute:*') {
        $key = $id.Substring(5)
        [AudioMixer]::ToggleAppMute($key)
        Draw-All
        return
    }
    if ($id -eq 'vol:master') {
        $v = [int][Math]::Round([double]$msg.value)
        [AudioMixer]::SetMasterVolume($v)
        $script:MasterVol = $v
        $script:MasterMuted = $false
        if ($kind -eq 'commit') { Draw-All }
        return
    }
    if ($id -like 'vol:*') {
        $key = $id.Substring(4)
        $v = [int][Math]::Round([double]$msg.value)
        [AudioMixer]::SetAppVolume($key, $v)
        if ($kind -eq 'commit') { Draw-All }
        return
    }
}

Draw-All

while ($true) {
    $line = $In.ReadLine()
    if ($null -eq $line) { break }
    if (-not $line.Trim()) { continue }
    try { $msg = ConvertFrom-Json $line } catch { continue }
    try {
        switch ([string]$msg.type) {
            'event' { Handle-Event $msg }
            'page_open' { Draw-All }
            'layout' {
                if ($msg.target -eq 'tile' -and $msg.size -and $msg.size -ne $script:TileSize) {
                    $script:TileSize = [string]$msg.size
                    Draw-Tile
                }
            }
            'shutdown' { break }
        }
    } catch {
        Log "error: $($_.Exception.Message)"
    }
}
