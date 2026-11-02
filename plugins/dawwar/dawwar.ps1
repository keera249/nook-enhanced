# Dawwar Integration & In-Notch Settings Plugin for Nook
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'

$script:DataDir = Join-Path $env:APPDATA 'Nook\plugin-data\dawwar'
$script:DawwarDir = Join-Path $env:LOCALAPPDATA 'Dawwar'
$script:SettingsPath = Join-Path $script:DawwarDir 'settings.json'
$script:TileSize = 'wide'
$script:CurrentTab = 0
$script:DroppedFiles = @()
$script:StatusMsg = ''
$script:StatusColor = 'accent'
$script:OpenRouterKeyInput = ''

$script:IconPath = Join-Path $PSScriptRoot 'dawwar-icon.png'
$script:DawwarIconBase64 = ''
if (Test-Path -LiteralPath $script:IconPath) {
    try {
        $bytes = [System.IO.File]::ReadAllBytes($script:IconPath)
        $script:DawwarIconBase64 = [Convert]::ToBase64String($bytes)
    } catch {}
}

$script:Themes = @('system', 'light', 'dark')
$script:ThemeLabels = @('System', 'Light', 'Dark')

$script:Accents = @('emerald', 'night', 'garnet', 'oud', 'plum', 'graphite')
$script:AccentLabels = @('Emerald', 'Night', 'Garnet', 'Oud', 'Plum', 'Graphite')

$script:AiProviders = @('openrouter', 'claude-cli', 'codex-cli')
$script:AiProviderLabels = @('OpenRouter', 'Claude CLI', 'Codex CLI')

$script:OpenRouterModels = @(
    'anthropic/claude-sonnet-5.5',
    'anthropic/claude-opus-5.5',
    'anthropic/claude-haiku-5.5',
    'google/gemini-3.1-flash-lite',
    'deepseek/deepseek-v4.1-flash',
    'qwen/qwen3.8-flash'
)
$script:OpenRouterModelLabels = @(
    'Sonnet 5.5',
    'Opus 5.5',
    'Haiku 5.5',
    'Gemini 3.1',
    'DeepSeek V4.1',
    'Qwen 3.8'
)

$script:ClaudeModels = @('sonnet', 'opus')
$script:ClaudeModelLabels = @('Claude Sonnet', 'Claude Opus')

$script:SpeechModels = @('auto', 'nvidia-ar', 'cohere-ar', 'nvidia-en')
$script:SpeechModelLabels = @('Auto', 'Nvidia AR', 'Cohere AR', 'Nvidia EN')

function Send-Message([hashtable]$Obj) {
    $json = $Obj | ConvertTo-Json -Depth 25 -Compress
    [Console]::Out.WriteLine($json)
    [Console]::Out.Flush()
}

function Get-DawwarExecutable {
    $candidates = @(
        (Join-Path $PSScriptRoot 'Dawwar.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Nook\Dawwar.exe'),
        'c:\Games\New folder\Dawwar.exe',
        'c:\Games\New folder\dawwar\Dawwar.exe',
        'c:\Games\New folder\Nook-Enhanced-Edition\Dawwar.exe',
        (Join-Path $env:USERPROFILE 'Downloads\Dawwar-0.2.3-x64.exe')
    )
    foreach ($c in $candidates) {
        if ($c -and (Test-Path -LiteralPath $c)) {
            return $c
        }
    }
    $dl = Get-ChildItem -Path (Join-Path $env:USERPROFILE 'Downloads') -Filter 'Dawwar*.exe' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($dl) { return $dl.FullName }
    return $null
}

function Get-DawwarProcess {
    $procs = @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
        $_.ProcessName -like 'Dawwar*'
    })
    if ($procs.Count -gt 0) { return $procs[0] }
    return $null
}

function Test-DawwarRunning {
    return ($null -ne (Get-DawwarProcess))
}

function Load-NookDawwarConfig {
    if (-not (Test-Path -LiteralPath $script:DataDir)) {
        New-Item -ItemType Directory -Force -Path $script:DataDir | Out-Null
    }
    $cfgFile = Join-Path $script:DataDir 'nook-dawwar.json'
    if (Test-Path -LiteralPath $cfgFile) {
        try {
            $raw = [System.IO.File]::ReadAllText($cfgFile, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
            if ($null -ne $raw.autoStartWithNook) {
                return @{ autoStartWithNook = [bool]$raw.autoStartWithNook }
            }
        } catch {}
    }
    return @{ autoStartWithNook = $true }
}

function Save-NookDawwarConfig([hashtable]$Cfg) {
    if (-not (Test-Path -LiteralPath $script:DataDir)) {
        New-Item -ItemType Directory -Force -Path $script:DataDir | Out-Null
    }
    $cfgFile = Join-Path $script:DataDir 'nook-dawwar.json'
    $json = $Cfg | ConvertTo-Json -Depth 5
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($cfgFile, $json, $utf8NoBom)
}

function Load-DawwarSettings {
    $defaults = [ordered]@{
        Paused           = $false
        StartWithWindows = $false
        Onboarded        = $true
        Theme            = 'system'
        Accent           = 'night'
        Glass            = $true
        WheelSound       = $true
        Version          = 2
        CheckUpdates     = $true
        LastUpdateCheck  = (Get-Date).ToString('o')
        WhatsNew         = $null
        SpeechModel      = 'auto'
        AiEnabled        = $true
        AiProvider       = 'openrouter'
        AiModel          = 'qwen/qwen3.8-flash'
        AiClaudeModel    = 'sonnet'
        AiNoConsent      = @()
    }
    if (Test-Path -LiteralPath $script:SettingsPath) {
        try {
            $raw = [System.IO.File]::ReadAllText($script:SettingsPath, [System.Text.Encoding]::UTF8) | ConvertFrom-Json
            foreach ($prop in $raw.PSObject.Properties) {
                $defaults[$prop.Name] = $prop.Value
            }
        } catch {}
    }
    return $defaults
}

function Save-DawwarSettings([System.Collections.Specialized.OrderedDictionary]$Settings, [bool]$RestartEngine = $true) {
    try {
        if (-not (Test-Path -LiteralPath $script:DawwarDir)) {
            New-Item -ItemType Directory -Force -Path $script:DawwarDir | Out-Null
        }

        $wasRunning = Test-DawwarRunning
        if ($wasRunning -and $RestartEngine) {
            Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'Dawwar*' } | Stop-Process -Force -ErrorAction SilentlyContinue
            Start-Sleep -Milliseconds 120
        }

        $json = $Settings | ConvertTo-Json -Depth 10
        $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($script:SettingsPath, $json, $utf8NoBom)

        $exe = Get-DawwarExecutable
        if ($exe) {
            $runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
            if ($Settings['StartWithWindows']) {
                Set-ItemProperty -Path $runKey -Name 'Dawwar' -Value "`"$exe`" --tray" -ErrorAction SilentlyContinue
            } else {
                Remove-ItemProperty -Path $runKey -Name 'Dawwar' -ErrorAction SilentlyContinue
            }
        }

        if ($RestartEngine -and (-not $Settings['Paused'])) {
            Start-DawwarEngine | Out-Null
        }
    } catch {}
}

function Start-DawwarEngine {
    $exe = Get-DawwarExecutable
    if (-not $exe) {
        $script:StatusMsg = 'Dawwar executable not found'
        $script:StatusColor = 'red'
        return $false
    }
    if (-not (Test-DawwarRunning)) {
        try {
            Start-Process -FilePath $exe -ArgumentList '--tray' -WindowStyle Hidden -ErrorAction SilentlyContinue | Out-Null
            Start-Sleep -Milliseconds 250
        } catch {}
    }
    return (Test-DawwarRunning)
}

function Stop-DawwarEngine {
    try {
        Get-Process -ErrorAction SilentlyContinue | Where-Object { $_.ProcessName -like 'Dawwar*' } | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 120
    } catch {}
}

function Open-DawwarNativeWindow {
    $exe = Get-DawwarExecutable
    if (-not $exe) { return }
    $signaled = $false
    try {
        $ev = [System.Threading.EventWaitHandle]::OpenExisting('Local\Dawwar.Show')
        if ($ev) {
            [void]$ev.Set()
            $ev.Dispose()
            $signaled = $true
        }
    } catch {}
    if (-not $signaled) {
        Start-Process -FilePath $exe -ErrorAction SilentlyContinue | Out-Null
    }
}

function Open-DawwarSamples {
    $samplesDir = Join-Path ([Environment]::GetFolderPath('Desktop')) 'Dawwar Samples'
    if (-not (Test-Path -LiteralPath $samplesDir)) {
        $samplesDir = Join-Path $env:TEMP 'Dawwar Samples'
        New-Item -ItemType Directory -Force -Path $samplesDir | Out-Null
    }
    $sampleTxt = Join-Path $samplesDir 'Dawwar Sample Notes.txt'
    if (-not (Test-Path -LiteralPath $sampleTxt)) {
        $utf8 = New-Object System.Text.UTF8Encoding($false)
        [System.IO.File]::WriteAllText($sampleTxt, "Dawwar Sample File`r`nDrag this file in Explorer and hold Shift, Ctrl, Space, or Alt to test the Dawwar wheel!", $utf8)
    }
    Start-Process -FilePath 'explorer.exe' -ArgumentList "`"$samplesDir`"" -ErrorAction SilentlyContinue | Out-Null
}

function Get-FileActions([string[]]$Files) {
    if (-not $Files -or $Files.Count -eq 0) { return @() }
    $first = $Files[0]
    $ext = ([System.IO.Path]::GetExtension($first)).ToLowerInvariant()
    $actions = @()

    if ($ext -in @('.png', '.jpg', '.jpeg', '.webp', '.heic', '.heif', '.bmp', '.tiff', '.tif', '.avif', '.gif')) {
        if ($ext -ne '.png') { $actions += @{ id = 'run:convert.png'; label = 'To PNG'; icon = 'file-png' } }
        if ($ext -notin @('.jpg', '.jpeg')) { $actions += @{ id = 'run:convert.jpg'; label = 'To JPG'; icon = 'file-jpg' } }
        $actions += @{ id = 'run:convert.pdf'; label = 'To PDF'; icon = 'file-pdf' }
        if ($Files.Count -gt 1) {
            $actions += @{ id = 'run:tool.images-to-pdf'; label = 'Merge to PDF'; icon = 'files' }
        }
        $actions += @{ id = 'run:tool.compress'; label = 'Compress'; icon = 'arrows-in-simple' }
        $actions += @{ id = 'run:tool.half-size'; label = 'Half Size'; icon = 'corners-in' }
        $actions += @{ id = 'run:tool.strip-metadata'; label = 'Strip GPS/Meta'; icon = 'eye-slash' }
        $actions += @{ id = 'run:tool.rotate'; label = 'Rotate 90 deg'; icon = 'arrow-clockwise' }
    } elseif ($ext -eq '.pdf') {
        $actions += @{ id = 'run:convert.docx'; label = 'To Word (DOCX)'; icon = 'file-doc' }
        $actions += @{ id = 'run:convert.jpg'; label = 'Pages to JPG'; icon = 'file-jpg' }
        $actions += @{ id = 'run:convert.png'; label = 'Pages to PNG'; icon = 'file-png' }
        $actions += @{ id = 'run:convert.txt'; label = 'Extract Text'; icon = 'file-txt' }
        $actions += @{ id = 'run:tool.compress-pdf'; label = 'Compress PDF'; icon = 'arrows-in-simple' }
        $actions += @{ id = 'run:tool.compress-pdf-strong'; label = 'Strong Compress'; icon = 'arrows-in' }
        $actions += @{ id = 'run:tool.split'; label = 'Split Pages'; icon = 'arrows-split' }
        if ($Files.Count -gt 1) {
            $actions += @{ id = 'run:tool.merge'; label = 'Merge PDFs'; icon = 'arrows-merge' }
        } else {
            $actions += @{ id = 'run:tool.merge-all'; label = 'Merge Folder PDFs'; icon = 'arrows-merge' }
        }
    } elseif ($ext -in @('.mp4', '.mov', '.mkv', '.webm', '.avi', '.wmv', '.m4v')) {
        $actions += @{ id = 'run:convert.mp3'; label = 'Extract MP3'; icon = 'file-audio' }
        if ($ext -ne '.mp4') { $actions += @{ id = 'run:convert.mp4'; label = 'To MP4'; icon = 'file-video' } }
        $actions += @{ id = 'run:convert.gif'; label = 'To GIF'; icon = 'gif' }
        $actions += @{ id = 'run:tool.messaging'; label = 'For Messaging'; icon = 'chat-circle' }
        $actions += @{ id = 'run:tool.small-video'; label = 'Small Size'; icon = 'arrows-in-simple' }
        $actions += @{ id = 'run:tool.transcribe'; label = 'Transcribe Text'; icon = 'closed-captioning' }
        $actions += @{ id = 'run:tool.transcribe-timed'; label = 'Subtitles (SRT)'; icon = 'subtitles' }
    } elseif ($ext -in @('.mp3', '.m4a', '.wav', '.flac', '.ogg', '.opus', '.wma', '.aac')) {
        if ($ext -ne '.mp3') { $actions += @{ id = 'run:convert.mp3'; label = 'To MP3'; icon = 'file-audio' } }
        if ($ext -ne '.wav') { $actions += @{ id = 'run:convert.wav'; label = 'To WAV'; icon = 'file-audio' } }
        if ($ext -ne '.m4a') { $actions += @{ id = 'run:convert.m4a'; label = 'To M4A'; icon = 'file-audio' } }
        $actions += @{ id = 'run:tool.audio-small'; label = 'Light Audio'; icon = 'arrows-in-simple' }
        $actions += @{ id = 'run:tool.transcribe'; label = 'Transcribe Text'; icon = 'closed-captioning' }
        $actions += @{ id = 'run:tool.transcribe-timed'; label = 'Subtitles (SRT)'; icon = 'subtitles' }
    } elseif ($ext -in @('.docx', '.doc', '.rtf', '.odt', '.md', '.html', '.htm', '.txt', '.srt', '.vtt', '.csv', '.tsv', '.xlsx')) {
        if ($ext -ne '.pdf') { $actions += @{ id = 'run:convert.pdf'; label = 'To PDF'; icon = 'file-pdf' } }
        if ($ext -ne '.txt') { $actions += @{ id = 'run:convert.txt'; label = 'To TXT'; icon = 'file-txt' } }
        $actions += @{ id = 'run:tool.fix-encoding'; label = 'Fix Arabic UTF-8'; icon = 'check-circle' }
        $actions += @{ id = 'run:tool.digits-latin'; label = 'Digits 123'; icon = 'hash' }
        $actions += @{ id = 'run:tool.digits-arabic'; label = 'Arabic Digits'; icon = 'hash-straight' }
        $actions += @{ id = 'run:tool.clean-arabic'; label = 'Clean Arabic'; icon = 'broom' }
        if ($ext -in @('.csv', '.tsv')) {
            $actions += @{ id = 'run:tool.excel-ready'; label = 'Excel Ready'; icon = 'file-xls' }
        }
    } elseif ($ext -in @('.zip', '.rar', '.7z', '.tar', '.gz')) {
        $actions += @{ id = 'run:tool.extract'; label = 'Extract Archive'; icon = 'folder-open' }
    }

    $actions += @{ id = 'run:tool.zip'; label = 'Compress ZIP'; icon = 'file-zip' }
    $actions += @{ id = 'run:action.copy-path'; label = 'Copy Path'; icon = 'copy' }
    $actions += @{ id = 'run:action.show'; label = 'Show in Folder'; icon = 'folder-open' }
    return $actions
}

function Invoke-DawwarAction([string]$ActionId, [string[]]$Files) {
    if (-not $Files -or $Files.Count -eq 0) { return }
    $exe = Get-DawwarExecutable
    if (-not $exe) {
        $script:StatusMsg = 'Dawwar.exe not found'
        $script:StatusColor = 'red'
        Update-AllViews
        return
    }
    $script:StatusMsg = "Running $ActionId..."
    $script:StatusColor = 'accent'
    Update-AllViews

    $quotedFiles = ($Files | ForEach-Object { "`"$_`"" }) -join ' '
    $argStr = "--run $ActionId $quotedFiles"
    $proc = Start-Process -FilePath $exe -ArgumentList $argStr -WindowStyle Hidden -Wait -PassThru -ErrorAction SilentlyContinue
    if ($proc -and $proc.ExitCode -eq 0) {
        $script:StatusMsg = "Completed ($ActionId) - Saved next to original file!"
        $script:StatusColor = 'green'
    } else {
        $script:StatusMsg = "Action finished ($ActionId)"
        $script:StatusColor = 'accent'
    }
    Update-AllViews
}

function Build-TileView {
    $settings = Load-DawwarSettings
    $running = Test-DawwarRunning
    $active = $running -and (-not [bool]$settings['Paused'])

    if ($script:TileSize -eq 'small') {
        return @{
            type = 'column'
            gap = 4
            align = 'center'
            justify = 'center'
            children = @(
                @{
                    type = 'image'
                    base64 = $script:DawwarIconBase64
                    path = $script:IconPath
                    width = 30
                    height = 30
                    rounding = 7
                },
                @{ type = 'text'; text = 'Dawwar'; size = 'small'; bold = $true }
            )
        }
    }

    if ($script:TileSize -eq 'large') {
        return @{
            type = 'column'
            gap = 8
            children = @(
                @{
                    type = 'row'
                    align = 'center'
                    gap = 10
                    children = @(
                        @{
                            type = 'image'
                            base64 = $script:DawwarIconBase64
                            path = $script:IconPath
                            width = 34
                            height = 34
                            rounding = 8
                        },
                        @{
                            type = 'column'
                            gap = 2
                            children = @(
                                @{ type = 'text'; text = 'Dawwar'; size = 'title'; bold = $true },
                                @{ type = 'text'; text = 'File Converter and Radial AI Wheel'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'icon'
                            icon = if ($active) { 'check-circle' } else { 'play-circle' }
                            size = 18
                            color = if ($active) { 'accent' } else { 'dim' }
                        }
                    )
                },
                @{
                    type = 'grid'
                    columns = 2
                    gap = 6
                    children = @(
                        @{
                            type = 'card'
                            padding = 7
                            children = @(
                                @{ type = 'text'; text = 'Shift + Drag'; size = 'small'; bold = $true; color = 'accent' },
                                @{ type = 'text'; text = 'Convert Format (PDF, Video, Image)'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{
                            type = 'card'
                            padding = 7
                            children = @(
                                @{ type = 'text'; text = 'Ctrl + Drag'; size = 'small'; bold = $true; color = 'accent' },
                                @{ type = 'text'; text = 'Tools (Compress, Merge, Transcribe)'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{
                            type = 'card'
                            padding = 7
                            children = @(
                                @{ type = 'text'; text = 'Space + Drag'; size = 'small'; bold = $true; color = 'accent' },
                                @{ type = 'text'; text = 'Actions (Copy Path, Move, Terminal)'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{
                            type = 'card'
                            padding = 7
                            children = @(
                                @{ type = 'text'; text = 'Alt + Drag (AI)'; size = 'small'; bold = $true; color = 'accent' },
                                @{ type = 'text'; text = if ($settings['AiEnabled']) { 'Enabled (Summarize, Translate, Slides)' } else { 'Off (Enable in Dawwar Settings)' }; size = 'small'; color = 'dim' }
                            )
                        }
                    )
                }
            )
        }
    }

    # Wide tile (default)
    return @{
        type = 'row'
        align = 'center'
        gap = 12
        children = @(
            @{
                type = 'image'
                base64 = $script:DawwarIconBase64
                path = $script:IconPath
                width = 38
                height = 38
                rounding = 9
            },
            @{
                type = 'column'
                gap = 2
                children = @(
                    @{ type = 'text'; text = 'Dawwar'; size = 'body'; bold = $true },
                    @{ type = 'text'; text = 'Shift | Ctrl | Space | Alt'; size = 'small'; color = 'dim' }
                )
            },
            @{ type = 'spacer' },
            @{
                type = 'icon'
                icon = if ($active) { 'check-circle' } else { 'play-circle' }
                size = 20
                color = if ($active) { 'accent' } else { 'dim' }
            }
        )
    }
}

function Build-PageView {
    $settings = Load-DawwarSettings
    $nookCfg = Load-NookDawwarConfig
    $running = Test-DawwarRunning
    $active = $running -and (-not [bool]$settings['Paused'])

    $topCard = @{
        type = 'card'
        padding = 10
        children = @(
            @{
                type = 'row'
                align = 'center'
                gap = 10
                children = @(
                    @{
                        type = 'image'
                        base64 = $script:DawwarIconBase64
                        path = $script:IconPath
                        width = 32
                        height = 32
                        rounding = 8
                    },
                    @{
                        type = 'column'
                        gap = 2
                        children = @(
                            @{ type = 'text'; text = 'Dawwar Background Wheel & Settings'; size = 'body'; bold = $true },
                            @{
                                type = 'text'
                                text = if ($active) { 'Running in background | Drag any file & hold Shift, Ctrl, Space, or Alt' } else { 'Wheel is currently paused | Toggle Active to enable background hooks' }
                                size = 'small'
                                color = if ($active) { 'accent' } else { 'dim' }
                            }
                        )
                    },
                    @{ type = 'spacer' },
                    @{
                        type = 'toggle'
                        id = 'toggle_engine'
                        value = [bool]$active
                        label = 'Active'
                    },
                    @{
                        type = 'toggle'
                        id = 'toggle_autostart'
                        value = [bool]$nookCfg.autoStartWithNook
                        label = 'Auto with Nook'
                    }
                )
            }
        )
    }

    $tabsRow = @{
        type = 'row'
        align = 'center'
        children = @(
            @{
                type = 'chips'
                id = 'nav_tab'
                options = @('Appearance & Wheel', 'AI & Speech', 'Quick Convert & Guide')
                selected = [int]$script:CurrentTab
            },
            @{ type = 'spacer' },
            @{
                type = 'button'
                id = 'btn_open_dawwar'
                label = 'Dawwar App'
                icon = 'app-window'
            }
        )
    }

    $tabContent = @()

    if ($script:CurrentTab -eq 0) {
        $themeIdx = [Math]::Max(0, [array]::IndexOf($script:Themes, [string]$settings['Theme']))
        $accentIdx = [Math]::Max(0, [array]::IndexOf($script:Accents, [string]$settings['Accent']))

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Appearance (Applies to Dawwar Wheel & Windows)'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'text'; text = 'Theme Mode'; size = 'body' },
                        @{ type = 'spacer' },
                        @{
                            type = 'chips'
                            id = 'chip_theme'
                            options = $script:ThemeLabels
                            selected = [int]$themeIdx
                        }
                    )
                },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'text'; text = 'Accent Palette'; size = 'body' },
                        @{ type = 'spacer' },
                        @{
                            type = 'chips'
                            id = 'chip_accent'
                            options = $script:AccentLabels
                            selected = [int]$accentIdx
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Glass Wheel & Surface Effect'; size = 'body' },
                                @{ type = 'text'; text = 'Translucent acrylic glass backdrop behind the radial wheel'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_glass'
                            value = [bool]$settings['Glass']
                        }
                    )
                }
            )
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Wheel Behavior & Startup'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Wheel Navigation Tick Sound'; size = 'body' },
                                @{ type = 'text'; text = 'Play subtle tactile click when hovering across wheel slices'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_sound'
                            value = [bool]$settings['WheelSound']
                        }
                    )
                },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Launch Dawwar with Windows'; size = 'body' },
                                @{ type = 'text'; text = 'Also register Dawwar in Windows startup (in addition to Nook auto-start)'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_win_startup'
                            value = [bool]$settings['StartWithWindows']
                        }
                    )
                },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Automatic Update Checks'; size = 'body' },
                                @{ type = 'text'; text = 'Check daily for signed Dawwar updates'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_updates'
                            value = [bool]$settings['CheckUpdates']
                        }
                    )
                }
            )
        }
    } elseif ($script:CurrentTab -eq 1) {
        $provIdx = [Math]::Max(0, [array]::IndexOf($script:AiProviders, [string]$settings['AiProvider']))
        $orModelIdx = [Math]::Max(0, [array]::IndexOf($script:OpenRouterModels, [string]$settings['AiModel']))
        $clModelIdx = [Math]::Max(0, [array]::IndexOf($script:ClaudeModels, [string]$settings['AiClaudeModel']))
        $spModelIdx = [Math]::Max(0, [array]::IndexOf($script:SpeechModels, [string]$settings['SpeechModel']))
        $hasOrKey = ($null -ne (cmdkey.exe /list | Select-String 'Dawwar/ai/openrouter'))
        $orStatusTxt = if ($hasOrKey) { 'Key Installed: Ready for Alt + Drag AI Wheel' } else { 'No Key: Paste API key below to activate Alt wheel' }
        $orStatusCol = if ($hasOrKey) { 'green' } else { 'dim' }

        $aiChildren = @(
            @{
                type = 'row'
                align = 'center'
                children = @(
                    @{
                        type = 'column'
                        gap = 1
                        children = @(
                            @{ type = 'text'; text = 'AI Wheel (Hold Alt while dragging)'; size = 'body'; bold = $true },
                            @{ type = 'text'; text = 'Summarize, translate, generate slides (PPTX), website, or video script'; size = 'small'; color = 'dim' }
                        )
                    },
                    @{ type = 'spacer' },
                    @{
                        type = 'toggle'
                        id = 'toggle_ai'
                        value = [bool]$settings['AiEnabled']
                    }
                )
            },
            @{ type = 'divider' },
            @{
                type = 'row'
                align = 'center'
                children = @(
                    @{ type = 'text'; text = 'AI Provider'; size = 'body' },
                    @{ type = 'spacer' },
                    @{
                        type = 'chips'
                        id = 'chip_ai_provider'
                        options = $script:AiProviderLabels
                        selected = [int]$provIdx
                    }
                )
            }
        )

        if ($settings['AiProvider'] -eq 'openrouter') {
            $aiChildren += @{
                type = 'column'
                gap = 4
                children = @(
                    @{ type = 'text'; text = 'OpenRouter Model'; size = 'small'; color = 'dim' },
                    @{
                        type = 'chips'
                        id = 'chip_or_model'
                        options = $script:OpenRouterModelLabels
                        selected = [int]$orModelIdx
                    },
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 6
                        children = @(
                            @{ type = 'icon'; icon = if ($hasOrKey) { 'check-circle' } else { 'key' }; size = 15; color = $orStatusCol },
                            @{ type = 'text'; text = $orStatusTxt; size = 'small'; color = $orStatusCol }
                        )
                    },
                    @{
                        type = 'input'
                        id = 'input_or_key'
                        hint = 'Paste OpenRouter Key here (starts with sk-or-...)'
                        value = $script:OpenRouterKeyInput
                        live = $true
                    },
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 8
                        children = @(
                            @{ type = 'button'; id = 'btn_save_key'; label = 'Save Key & Activate'; icon = 'check'; primary = $true },
                            @{ type = 'button'; id = 'btn_delete_key'; label = 'Remove Key'; icon = 'trash' },
                            @{ type = 'spacer' },
                            @{ type = 'button'; id = 'btn_openrouter_keys'; label = 'Get API Key'; icon = 'arrow-square-out' }
                        )
                    }
                )
            }
        } elseif ($settings['AiProvider'] -eq 'claude-cli') {
            $aiChildren += @{
                type = 'row'
                align = 'center'
                children = @(
                    @{ type = 'text'; text = 'Claude Code Model'; size = 'body' },
                    @{ type = 'spacer' },
                    @{
                        type = 'chips'
                        id = 'chip_claude_model'
                        options = $script:ClaudeModelLabels
                        selected = [int]$clModelIdx
                    }
                )
            }
        } else {
            $aiChildren += @{
                type = 'text'
                text = 'Uses your installed OpenAI Codex CLI with its default model in read-only mode.'
                size = 'small'
                color = 'dim'
            }
        }

        if ($script:StatusMsg) {
            $aiChildren += @{
                type = 'text'
                text = $script:StatusMsg
                size = 'small'
                color = $script:StatusColor
            }
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = $aiChildren
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Offline Speech-to-Text (Hold Ctrl on Audio/Video)'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'text'; text = 'Transcription Model'; size = 'body' },
                        @{ type = 'spacer' },
                        @{
                            type = 'chips'
                            id = 'chip_speech_model'
                            options = $script:SpeechModelLabels
                            selected = [int]$spModelIdx
                        }
                    )
                },
                @{
                    type = 'text'
                    text = 'Runs 100% offline on your PC using ONNX Runtime. Hold Ctrl while dragging audio or video to transcribe.'
                    size = 'small'
                    color = 'dim'
                }
            )
        }
    } else {
        # Tab 2: Quick Convert & Wheel Guide
        if ($script:DroppedFiles -and $script:DroppedFiles.Count -gt 0) {
            $firstFile = $script:DroppedFiles[0]
            $fileName = [System.IO.Path]::GetFileName($firstFile)
            $countTxt = if ($script:DroppedFiles.Count -gt 1) { "$fileName (+$($script:DroppedFiles.Count - 1) more)" } else { $fileName }
            $actions = Get-FileActions $script:DroppedFiles
            $btnNodes = @()
            foreach ($act in $actions) {
                $btnNodes += @{
                    type = 'button'
                    id = $act.id
                    label = $act.label
                    icon = $act.icon
                }
            }

            $dropChildren = @(
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'icon'; icon = 'file-arrow-down'; size = 18; color = 'accent' },
                        @{ type = 'text'; text = "Dropped: $countTxt"; size = 'body'; bold = $true },
                        @{ type = 'spacer' },
                        @{ type = 'button'; id = 'btn_clear_drop'; label = 'Clear'; icon = 'x' }
                    )
                }
            )
            if ($script:StatusMsg) {
                $dropChildren += @{
                    type = 'text'
                    text = $script:StatusMsg
                    size = 'small'
                    color = $script:StatusColor
                }
            }
            $dropChildren += @{
                type = 'grid'
                columns = 3
                gap = 6
                children = $btnNodes
            }

            $tabContent += @{
                type = 'card'
                padding = 10
                gap = 8
                children = $dropChildren
            }
        } else {
            $tabContent += @{
                type = 'card'
                padding = 10
                gap = 6
                children = @(
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 8
                        children = @(
                            @{ type = 'icon'; icon = 'hand-grabbing'; size = 18; color = 'accent' },
                            @{ type = 'text'; text = 'Direct Notch Drop Converter'; size = 'body'; bold = $true },
                            @{ type = 'spacer' },
                            @{ type = 'button'; id = 'btn_samples'; label = 'Open Sample Files'; icon = 'folder-open' }
                        )
                    },
                    @{
                        type = 'text'
                        text = 'Drop any file (PDF, image, video, audio, Word, CSV, SRT) onto the Dawwar tile or this notch window to convert it immediately!'
                        size = 'small'
                        color = 'dim'
                        wrap = $true
                    }
                )
            }
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 6
            children = @(
                @{ type = 'text'; text = 'Dawwar Global Wheel Shortcuts (Drag Any File in Windows)'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'grid'
                    columns = 2
                    gap = 6
                    children = @(
                        @{
                            type = 'column'
                            gap = 2
                            children = @(
                                @{ type = 'text'; text = '1. Hold [Shift] while dragging'; size = 'small'; bold = $true },
                                @{ type = 'text'; text = 'Convert format: PDF to Word/JPG/PNG, Video to MP4/MP3/GIF, Image to PNG/JPG/PDF'; size = 'small'; color = 'dim'; wrap = $true }
                            )
                        },
                        @{
                            type = 'column'
                            gap = 2
                            children = @(
                                @{ type = 'text'; text = '2. Hold [Ctrl] while dragging'; size = 'small'; bold = $true },
                                @{ type = 'text'; text = 'Smart Tools: Compress, Half-Size, Offline Speech-to-Text, Merge/Split PDF, Fix Arabic UTF-8'; size = 'small'; color = 'dim'; wrap = $true }
                            )
                        },
                        @{
                            type = 'column'
                            gap = 2
                            children = @(
                                @{ type = 'text'; text = '3. Hold [Space] while dragging'; size = 'small'; bold = $true },
                                @{ type = 'text'; text = 'Quick Actions: Copy Path, Copy/Move To Folder, Open Terminal Here, Show in Explorer'; size = 'small'; color = 'dim'; wrap = $true }
                            )
                        },
                        @{
                            type = 'column'
                            gap = 2
                            children = @(
                                @{ type = 'text'; text = '4. Hold [Alt] while dragging'; size = 'small'; bold = $true },
                                @{ type = 'text'; text = 'AI Wheel: Summarize to Word, Translate AR/EN, Presentation Slides (PPTX), Website, Video Script'; size = 'small'; color = 'dim'; wrap = $true }
                            )
                        }
                    )
                }
            )
        }
    }

    return @{
        type = 'column'
        scroll = $true
        gap = 8
        children = @($topCard, $tabsRow) + $tabContent
    }
}

function Update-AllViews {
    Send-Message @{
        type = 'tile'
        view = (Build-TileView)
    }
    Send-Message @{
        type = 'page'
        view = (Build-PageView)
        actions = @(
            @{ id = 'act_open_native'; icon = 'app-window'; tooltip = 'Open Dawwar Native Window' },
            @{ id = 'act_samples'; icon = 'folder-open'; tooltip = 'Open Dawwar Sample Files' },
            @{ id = 'act_restart'; icon = 'arrows-clockwise'; tooltip = 'Restart Dawwar Background Engine' }
        )
    }
}

# Initial auto-start check on plugin load
$initCfg = Load-NookDawwarConfig
if ($initCfg.autoStartWithNook) {
    $initSettings = Load-DawwarSettings
    if (-not $initSettings['Paused']) {
        Start-DawwarEngine | Out-Null
    }
}
Update-AllViews

while ($null -ne ($line = [Console]::In.ReadLine())) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    try {
        $msg = $line | ConvertFrom-Json
    } catch {
        continue
    }

    switch ($msg.type) {
        'hello' {
            if ($msg.data_dir) {
                $script:DataDir = $msg.data_dir
            }
            $nookCfg = Load-NookDawwarConfig
            if ($nookCfg.autoStartWithNook) {
                $s = Load-DawwarSettings
                if (-not $s['Paused']) {
                    Start-DawwarEngine | Out-Null
                }
            }
            Update-AllViews
        }
        'layout' {
            if ($msg.target -eq 'tile' -and $msg.size) {
                $script:TileSize = [string]$msg.size
                Send-Message @{ type = 'tile'; view = (Build-TileView) }
            }
        }
        'page_open' {
            Update-AllViews
        }
        'files' {
            if ($msg.paths) {
                $script:DroppedFiles = @($msg.paths | ForEach-Object { [string]$_ })
                $script:CurrentTab = 2
                $script:StatusMsg = 'Select an action below to run on your dropped file(s)'
                $script:StatusColor = 'accent'
                Update-AllViews
                Send-Message @{ type = 'show' }
                Send-Message @{ type = 'open_page' }
            }
        }
        'event' {
            $id = [string]$msg.id
            $val = $msg.value
            $settings = Load-DawwarSettings

            if ($id -eq 'tile_toggle_engine' -or $id -eq 'toggle_engine') {
                $running = Test-DawwarRunning
                $currentlyActive = $running -and (-not [bool]$settings['Paused'])
                $newActive = if ($id -eq 'toggle_engine' -and $null -ne $val) { [bool]$val } else { -not $currentlyActive }
                if ($newActive) {
                    $settings['Paused'] = $false
                    Save-DawwarSettings $settings $true
                } else {
                    $settings['Paused'] = $true
                    Save-DawwarSettings $settings $false
                    Stop-DawwarEngine
                }
                Update-AllViews
            } elseif ($id -eq 'toggle_autostart') {
                $nookCfg = Load-NookDawwarConfig
                $nookCfg.autoStartWithNook = [bool]$val
                Save-NookDawwarConfig $nookCfg
                if ($nookCfg.autoStartWithNook -and (-not (Test-DawwarRunning))) {
                    $settings['Paused'] = $false
                    Save-DawwarSettings $settings $true
                }
                Update-AllViews
            } elseif ($id -eq 'nav_tab') {
                $script:CurrentTab = [int]$val
                $script:StatusMsg = ''
                Update-AllViews
            } elseif ($id -eq 'btn_open_dawwar' -or $id -eq 'act_open_native') {
                Open-DawwarNativeWindow
            } elseif ($id -eq 'btn_samples' -or $id -eq 'act_samples') {
                Open-DawwarSamples
            } elseif ($id -eq 'act_restart') {
                Stop-DawwarEngine
                $settings['Paused'] = $false
                Save-DawwarSettings $settings $true
                $script:StatusMsg = 'Dawwar restarted.'
                $script:StatusColor = 'green'
                Update-AllViews
            } elseif ($id -eq 'chip_theme') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:Themes.Count) {
                    $settings['Theme'] = $script:Themes[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'chip_accent') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:Accents.Count) {
                    $settings['Accent'] = $script:Accents[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'toggle_glass') {
                $settings['Glass'] = [bool]$val
                Save-DawwarSettings $settings $true
                Update-AllViews
            } elseif ($id -eq 'toggle_sound') {
                $settings['WheelSound'] = [bool]$val
                Save-DawwarSettings $settings $true
                Update-AllViews
            } elseif ($id -eq 'toggle_win_startup') {
                $settings['StartWithWindows'] = [bool]$val
                Save-DawwarSettings $settings $false
                Update-AllViews
            } elseif ($id -eq 'toggle_updates') {
                $settings['CheckUpdates'] = [bool]$val
                Save-DawwarSettings $settings $false
                Update-AllViews
            } elseif ($id -eq 'toggle_ai') {
                $settings['AiEnabled'] = [bool]$val
                Save-DawwarSettings $settings $true
                Update-AllViews
            } elseif ($id -eq 'chip_ai_provider') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:AiProviders.Count) {
                    $settings['AiProvider'] = $script:AiProviders[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'chip_or_model') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:OpenRouterModels.Count) {
                    $settings['AiModel'] = $script:OpenRouterModels[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'chip_claude_model') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:ClaudeModels.Count) {
                    $settings['AiClaudeModel'] = $script:ClaudeModels[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'chip_speech_model') {
                $idx = [int]$val
                if ($idx -ge 0 -and $idx -lt $script:SpeechModels.Count) {
                    $settings['SpeechModel'] = $script:SpeechModels[$idx]
                    Save-DawwarSettings $settings $true
                    Update-AllViews
                }
            } elseif ($id -eq 'input_or_key') {
                $script:OpenRouterKeyInput = [string]$val
            } elseif ($id -eq 'btn_save_key') {
                if ($script:OpenRouterKeyInput -and $script:OpenRouterKeyInput.Trim().Length -gt 5) {
                    $cleanKey = $script:OpenRouterKeyInput.Trim()
                    cmdkey.exe /generic:Dawwar/ai/openrouter /user:openrouter /pass:$cleanKey | Out-Null
                    $settings['AiEnabled'] = $true
                    Save-DawwarSettings $settings $true
                    $script:StatusMsg = 'OpenRouter API key saved! Alt + Drag AI wheel is now active.'
                    $script:StatusColor = 'green'
                    Update-AllViews
                } else {
                    $script:StatusMsg = 'Please paste a valid key starting with sk-or-'
                    $script:StatusColor = 'red'
                    Update-AllViews
                }
            } elseif ($id -eq 'btn_delete_key') {
                cmdkey.exe /delete:Dawwar/ai/openrouter | Out-Null
                Stop-DawwarEngine
                Start-DawwarEngine
                $script:OpenRouterKeyInput = ''
                $script:StatusMsg = 'OpenRouter key removed.'
                $script:StatusColor = 'dim'
                Update-AllViews
            } elseif ($id -eq 'btn_openrouter_keys') {
                Send-Message @{ type = 'open'; target = 'https://openrouter.ai/keys' }
            } elseif ($id -eq 'btn_clear_drop') {
                $script:DroppedFiles = @()
                $script:StatusMsg = ''
                Update-AllViews
            } elseif ($id.StartsWith('run:')) {
                $actId = $id.Substring(4)
                Invoke-DawwarAction $actId $script:DroppedFiles
            }
        }
    }
}
