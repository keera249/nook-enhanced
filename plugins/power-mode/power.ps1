# Power & Performance Switcher - a Nook plugin (live mode, PowerShell 5.1+).
# Controls Windows 10/11 Power Mode Overlay (Best Performance / Balanced / Power Saver)
# AND Windows Power Plans (Ultimate Performance, High Performance, Balanced, Power Saver).

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Utf8 = New-Object System.Text.UTF8Encoding $false
$Out = [System.IO.StreamWriter]::new([Console]::OpenStandardOutput(), $Utf8)
$Out.AutoFlush = $true
$In = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), $Utf8)

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class WinPower
{
    [DllImport("powrprof.dll")]
    public static extern uint PowerGetEffectiveOverlayScheme(out Guid EffectiveOverlayPolicyGuid);

    [DllImport("powrprof.dll", EntryPoint = "PowerSetActiveOverlayScheme")]
    public static extern uint PowerSetActiveOverlayScheme(Guid OverlaySchemeGuid);
}
"@

$GuidSaver    = [Guid]'961cc777-2547-4f9d-8174-7d86181b8a7a'
$GuidBalanced = [Guid]'00000000-0000-0000-0000-000000000000'
$GuidPerf     = [Guid]'ded574b5-45a0-4f42-8737-46345c09c238'

$script:TileSize = 'wide'
$script:OverlayMode = 1   # 0 = Power Saver, 1 = Balanced, 2 = Best Performance
$script:Plans = @()
$script:Status = $null

function Send($msg) {
    $Out.WriteLine((ConvertTo-Json -InputObject $msg -Depth 40 -Compress))
}

function Log([string]$text) {
    [Console]::Error.WriteLine("[power-mode] $text")
}

function New-List { , ([System.Collections.Generic.List[object]]::new()) }

function Refresh-Power {
    $g = [Guid]::Empty
    try {
        if ([WinPower]::PowerGetEffectiveOverlayScheme([ref]$g) -eq 0) {
            if ($g -eq $GuidSaver)    { $script:OverlayMode = 0 }
            elseif ($g -eq $GuidPerf) { $script:OverlayMode = 2 }
            else                      { $script:OverlayMode = 1 }
        }
    } catch { }

    $list = New-List
    try {
        $lines = & powercfg.exe /list 2>$null
        foreach ($ln in $lines) {
            if ($ln -match '([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})\s+\(([^)]+)\)(\s+\*)?') {
                $guid = $Matches[1].ToLowerInvariant()
                $name = $Matches[2].Trim()
                $active = [bool]$Matches[3]
                $list.Add([pscustomobject]@{
                    guid   = $guid
                    name   = $name
                    active = $active
                })
            }
        }
    } catch { }
    $script:Plans = @($list.ToArray())
}

function Set-OverlayMode([int]$idx) {
    $target = switch ($idx) {
        0 { $GuidSaver }
        2 { $GuidPerf }
        default { $GuidBalanced }
    }
    try {
        $r = [WinPower]::PowerSetActiveOverlayScheme($target)
        if ($r -eq 0) {
            $script:OverlayMode = $idx
            $label = @('Power Saver', 'Balanced', 'Best Performance')[$idx]
            $script:Status = @{ text = "Switched Power Mode to $label"; color = 'green' }
        } else {
            $script:Status = @{ text = "Power overlay returned code $r"; color = 'orange' }
        }
    } catch {
        $script:Status = @{ text = $_.Exception.Message; color = 'red' }
    }
}

function Set-PowerPlan([string]$guid) {
    try {
        & powercfg.exe /setactive $guid | Out-Null
        # If switching to a High/Ultimate plan, also set overlay to Best Performance.
        foreach ($p in $script:Plans) {
            if ($p.guid -eq $guid) {
                if ($p.name -match '(?i)(ultimate|high)') { [void][WinPower]::PowerSetActiveOverlayScheme($GuidPerf) }
                elseif ($p.name -match '(?i)saver')       { [void][WinPower]::PowerSetActiveOverlayScheme($GuidSaver) }
                $script:Status = @{ text = "Activated plan: $($p.name)"; color = 'green' }
            }
        }
    } catch {
        $script:Status = @{ text = $_.Exception.Message; color = 'red' }
    }
}

function Unlock-PerformancePlans {
    $templates = @(
        @{ guid = 'e9a42b02-d5df-448d-aa00-03f14749eb61'; pat = '(?i)ultimate' },
        @{ guid = '8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c'; pat = '(?i)high performance' },
        @{ guid = 'a1841308-3541-4fab-bc81-f71556f20b4a'; pat = '(?i)power saver' }
    )
    $added = 0
    foreach ($t in $templates) {
        $exists = $false
        foreach ($p in $script:Plans) {
            if ($p.name -match $t.pat) { $exists = $true; break }
        }
        if (-not $exists) {
            & powercfg.exe -duplicatescheme $t.guid 2>$null | Out-Null
            $added++
        }
    }
    Refresh-Power
    if ($added -gt 0) {
        $script:Status = @{ text = "Unlocked performance power plans!"; color = 'green' }
    } else {
        $script:Status = @{ text = "All power plans are already unlocked"; color = 'dim' }
    }
}

function Is-PerfActive {
    if ($script:OverlayMode -eq 2) { return $true }
    foreach ($p in $script:Plans) {
        if ($p.active -and $p.name -match '(?i)(ultimate|high)') { return $true }
    }
    return $false
}

function Draw-Tile {
    $perf = Is-PerfActive
    Send @{ type = 'active'; active = [bool]$perf }

    $modeLabel = switch ($script:OverlayMode) {
        0 { 'Power Saver' }
        2 { 'Best Performance' }
        default {
            $activePlan = ($script:Plans | Where-Object { $_.active } | Select-Object -First 1)
            if ($activePlan -and $activePlan.name -notmatch '(?i)^balanced$') { $activePlan.name } else { 'Balanced' }
        }
    }
    $icon = if ($perf) { 'lightning' } elseif ($script:OverlayMode -eq 0) { 'battery-plus' } else { 'gauge' }
    $color = if ($perf) { 'orange' } elseif ($script:OverlayMode -eq 0) { 'green' } else { 'accent' }

    switch ($script:TileSize) {
        'small' {
            Send @{
                type = 'tile'
                view = @{
                    type = 'column'; align = 'center'; justify = 'center'
                    children = @(
                        @{ type = 'button'; id = 'toggle_turbo'; icon = $icon; primary = $perf; tooltip = "Power: $modeLabel (click to toggle Gaming Performance)" }
                    )
                }
            }
        }
        default {
            Send @{
                type = 'tile'
                view = @{
                    type = 'row'; align = 'center'; gap = 8
                    children = @(
                        @{ type = 'icon'; icon = $icon; size = 22; color = $color; fill = $perf },
                        @{
                            type = 'column'; gap = 0
                            children = @(
                                @{ type = 'text'; text = 'Power Mode'; size = 'small'; color = 'dim' },
                                @{ type = 'text'; text = $modeLabel; size = 'body'; bold = $true }
                            )
                        },
                        @{ type = 'spacer' },
                        @{ type = 'button'; id = 'toggle_turbo'; icon = 'lightning'; primary = $perf; tooltip = 'Toggle Gaming Best Performance' }
                    )
                }
            }
        }
    }
}

function Draw-Page {
    $kids = New-List

    # Section 1: Windows 10/11 Power Mode Overlay (Instant Switch)
    $kids.Add(@{
        type = 'card'; padding = 10; gap = 8
        children = @(
            @{
                type = 'row'; align = 'center'; gap = 8
                children = @(
                    @{ type = 'icon'; icon = 'lightning'; size = 20; color = $(if ($script:OverlayMode -eq 2) { 'orange' } else { 'accent' }); fill = ($script:OverlayMode -eq 2) },
                    @{ type = 'text'; text = 'Windows Performance Mode'; size = 'title' },
                    @{ type = 'spacer' },
                    @{ type = 'button'; id = 'toggle_turbo'; label = $(if ($script:OverlayMode -eq 2) { 'Turbo ON' } else { 'Boost Now' }); icon = 'fire'; primary = ($script:OverlayMode -eq 2) }
                )
            },
            @{
                type = 'chips'
                id = 'overlay_mode'
                options = @('Power Saver', 'Balanced', 'Best Performance')
                selected = [int]$script:OverlayMode
            }
        )
    })

    if ($script:Status) {
        $kids.Add(@{ type = 'text'; text = $script:Status.text; size = 'small'; color = $script:Status.color })
    }

    # Section 2: Classic Power Plans (powercfg)
    $items = New-List
    foreach ($p in $script:Plans) {
        $pIcon = if ($p.name -match '(?i)ultimate') { 'fire' }
                 elseif ($p.name -match '(?i)high') { 'lightning' }
                 elseif ($p.name -match '(?i)saver') { 'battery-plus' }
                 else { 'gauge' }
        $items.Add(@{
            id = "plan:$($p.guid)"
            title = $p.name
            icon = $pIcon
            trailing = $(if ($p.active) { 'Active' } else { '' })
            selected = [bool]$p.active
        })
    }

    $planKids = New-List
    $planKids.Add(@{
        type = 'row'; align = 'center'; gap = 8
        children = @(
            @{ type = 'icon'; icon = 'cpu'; size = 18; color = 'accent' },
            @{ type = 'text'; text = 'Power Plans (powercfg)'; size = 'body'; bold = $true },
            @{ type = 'spacer' },
            @{ type = 'button'; id = 'unlock_plans'; label = 'Unlock Ultimate Plan'; icon = 'plus'; tooltip = 'Unlock hidden High & Ultimate Performance plans' }
        )
    })
    if ($items.Count -gt 0) {
        $planKids.Add(@{ type = 'list'; items = $items })
    }

    $kids.Add(@{ type = 'card'; padding = 10; gap = 6; children = $planKids })

    Send @{
        type = 'page'
        actions = @(
            @{ id = 'refresh'; icon = 'arrow-clockwise'; tooltip = 'Refresh power state' },
            @{ id = 'open_cpl'; icon = 'gear'; tooltip = 'Open Windows Power Options' }
        )
        view = @{ type = 'column'; gap = 8; scroll = $true; children = $kids }
    }
}

function Draw-All {
    Refresh-Power
    Draw-Tile
    Draw-Page
}

function Handle-Event($msg) {
    $id = [string]$msg.id
    if ($id -eq 'toggle_turbo') {
        $next = if ($script:OverlayMode -eq 2) { 1 } else { 2 }
        Set-OverlayMode $next
        Draw-All
    }
    elseif ($id -eq 'overlay_mode') {
        $idx = [int]$msg.value.index
        Set-OverlayMode $idx
        Draw-All
    }
    elseif ($id -like 'plan:*') {
        $guid = $id.Substring(5)
        Set-PowerPlan $guid
        Draw-All
    }
    elseif ($id -eq 'unlock_plans') {
        Unlock-PerformancePlans
        Draw-All
    }
    elseif ($id -eq 'open_cpl') {
        Send @{ type = 'open'; target = 'powercfg.cpl' }
    }
    elseif ($id -eq 'refresh') {
        $script:Status = $null
        Draw-All
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
            'event'      { Handle-Event $msg }
            'page_open'  { $script:Status = $null; Draw-All }
            'layout'     {
                if ($msg.target -eq 'tile' -and $msg.size -and $msg.size -ne $script:TileSize) {
                    $script:TileSize = [string]$msg.size
                    Draw-Tile
                }
            }
            'shutdown'   { break }
        }
    } catch {
        Log "error: $($_.Exception.Message)"
    }
}
