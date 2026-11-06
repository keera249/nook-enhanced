# Nook Prayer Times, Azan Countdown and Athkar Plugin
# Dynamic island notch countdown ring, Bento tile, and full in-notch settings with scheduled Athkar.

[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$ErrorActionPreference = 'SilentlyContinue'

$script:DataDir = Join-Path $env:APPDATA 'Nook\plugin-data\prayer-times'
if (-not (Test-Path -LiteralPath $script:DataDir)) {
    New-Item -ItemType Directory -Force -Path $script:DataDir | Out-Null
}

$script:SettingsFile = Join-Path $script:DataDir 'settings.json'
$script:LocationFile = Join-Path $script:DataDir 'location.json'

$script:IconPath = Join-Path $PSScriptRoot 'prayer-icon.png'
$script:PrayerIconBase64 = ''
if (Test-Path -LiteralPath $script:IconPath) {
    try {
        $bytes = [System.IO.File]::ReadAllBytes($script:IconPath)
        $script:PrayerIconBase64 = [Convert]::ToBase64String($bytes)
    } catch {}
}

# Load Athkar Database from athkar.json
$script:AthkarFile = Join-Path $PSScriptRoot 'athkar.json'
$script:AthkarList = @()
if (Test-Path -LiteralPath $script:AthkarFile) {
    try {
        $script:AthkarList = Get-Content -LiteralPath $script:AthkarFile -Raw -Encoding UTF8 | ConvertFrom-Json
    } catch {}
}

# C# Background Stdin Reader so the 1-second timer loop never blocks on ReadLine
Add-Type -TypeDefinition @"
using System;
using System.Collections.Concurrent;
using System.Threading;

public static class PrayerStdinReader
{
    private static ConcurrentQueue<string> _queue = new ConcurrentQueue<string>();
    private static Thread _thread;
    private static bool _started = false;

    public static void Start()
    {
        if (_started) return;
        _started = true;
        _thread = new Thread(() =>
        {
            string line;
            while ((line = Console.ReadLine()) != null)
            {
                _queue.Enqueue(line);
            }
        });
        _thread.IsBackground = true;
        _thread.Start();
    }

    public static string TryReadLine()
    {
        string line;
        if (_queue.TryDequeue(out line)) return line;
        return null;
    }
}
"@

[PrayerStdinReader]::Start()

# Helper for chips index conversion
function Get-IndexValue($v) {
    if ($null -eq $v) { return 0 }
    if ($null -ne $v.index) { return [int]$v.index }
    try { return [int]$v } catch { return 0 }
}

function Send-Msg($obj) {
    $json = $obj | ConvertTo-Json -Depth 30 -Compress
    [Console]::Out.WriteLine($json)
    [Console]::Out.Flush()
}

# Settings definitions
$script:ThresholdOptions = @('30 min (Default)', '15 min', '45 min', '60 min', 'Always')
$script:ThresholdValues  = @(30, 15, 45, 60, 0)

$script:MethodLabels = @('Umm Al-Qura (Makkah)', 'Egyptian Authority', 'Muslim World League', 'ISNA (North America)', 'Karachi')
$script:MethodIds    = @(4, 5, 3, 2, 1)

$script:AthkarIntervalOptions = @('10 min', '15 min', '20 min', '30 min (Default)', '45 min', '1 Hour', '2 Hours', '3 Hours')
$script:AthkarIntervalValues  = @(10, 15, 20, 30, 45, 60, 120, 180)

$script:AthkarLangOptions = @('Bilingual (Dhikr + Meaning)', 'Arabic Short', 'English / Transliteration')
$script:AthkarLangValues  = @(1, 0, 2)

function Load-Settings {
    $defaults = [ordered]@{
        ShowBeforeMinutes = 30
        AutoLocation      = $true
        ManualCity        = ''
        ManualCountry     = ''
        CalcMethod        = 4
        AzanAudio         = $true
        DesktopNotify     = $true
        LanguageMode      = 0
        AthkarEnabled     = $true
        AthkarIntervalMin = 30
        AthkarLanguage    = 1
        AthkarAudio       = $true
        AthkarToast       = $true
    }
    if (Test-Path -LiteralPath $script:SettingsFile) {
        try {
            $raw = Get-Content -LiteralPath $script:SettingsFile -Raw -Encoding UTF8 | ConvertFrom-Json
            foreach ($prop in $raw.PSObject.Properties) {
                $defaults[$prop.Name] = $prop.Value
            }
        } catch {}
    }
    return $defaults
}

function Save-Settings($s) {
    try {
        $json = $s | ConvertTo-Json -Depth 5
        [System.IO.File]::WriteAllText($script:SettingsFile, $json, [System.Text.Encoding]::UTF8)
    } catch {}
}

# Geolocation Lookup & Caching
function Get-UserLocation([bool]$forceRefresh = $false) {
    if (-not $forceRefresh -and (Test-Path -LiteralPath $script:LocationFile)) {
        try {
            $cached = Get-Content -LiteralPath $script:LocationFile -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($cached.lat -and $cached.lon) {
                return $cached
            }
        } catch {}
    }

    $loc = $null
    try {
        $res = Invoke-RestMethod -Uri "http://ip-api.com/json" -TimeoutSec 4
        if ($res.status -eq 'success') {
            $loc = [PSCustomObject]@{
                city     = [string]$res.city
                country  = [string]$res.country
                lat      = [double]$res.lat
                lon      = [double]$res.lon
                timezone = [string]$res.timezone
                updated  = (Get-Date).ToString('yyyy-MM-dd HH:mm')
            }
        }
    } catch {}

    if (-not $loc) {
        try {
            $res2 = Invoke-RestMethod -Uri "https://ipapi.co/json/" -TimeoutSec 4
            if ($res2.latitude) {
                $loc = [PSCustomObject]@{
                    city     = [string]$res2.city
                    country  = [string]$res2.country_name
                    lat      = [double]$res2.latitude
                    lon      = [double]$res2.longitude
                    timezone = [string]$res2.timezone
                    updated  = (Get-Date).ToString('yyyy-MM-dd HH:mm')
                }
            }
        } catch {}
    }

    if (-not $loc) {
        $loc = [PSCustomObject]@{
            city     = 'Riyadh'
            country  = 'Saudi Arabia'
            lat      = 24.6869
            lon      = 46.7224
            timezone = 'Asia/Riyadh'
            updated  = (Get-Date).ToString('yyyy-MM-dd HH:mm')
        }
    }

    try {
        $json = $loc | ConvertTo-Json
        [System.IO.File]::WriteAllText($script:LocationFile, $json, [System.Text.Encoding]::UTF8)
    } catch {}

    return $loc
}

# Offline Astronomical Prayer Calculation Fallback
function Calculate-OfflineTimes([double]$lat, [double]$lon, [double]$tzOffset, [datetime]$date, [int]$method) {
    $fajrAngle = 18.5
    $ishaAngle = 0.0
    $ishaInterval = 90
    if ($method -eq 5) { $fajrAngle = 19.5; $ishaAngle = 17.5; $ishaInterval = 0 }
    elseif ($method -eq 3) { $fajrAngle = 18.0; $ishaAngle = 17.0; $ishaInterval = 0 }
    elseif ($method -eq 2) { $fajrAngle = 15.0; $ishaAngle = 15.0; $ishaInterval = 0 }
    elseif ($method -eq 1) { $fajrAngle = 18.0; $ishaAngle = 18.0; $ishaInterval = 0 }

    $deg2rad = [Math]::PI / 180.0
    $rad2deg = 180.0 / [Math]::PI

    $y = $date.Year
    $m = $date.Month
    $d = $date.Day
    if ($m -le 2) { $y -= 1; $m += 12 }
    $a = [Math]::Floor($y / 100.0)
    $b = 2 - $a + [Math]::Floor($a / 4.0)
    $jd = [Math]::Floor(365.25 * ($y + 4716)) + [Math]::Floor(30.6001 * ($m + 1)) + $d + $b - 1524.5
    $d0 = $jd - 2451545.0

    $g = (357.529 + 0.98560028 * $d0) % 360.0
    $q = (280.459 + 0.98564736 * $d0) % 360.0
    $L = ($q + 1.915 * [Math]::Sin($g * $deg2rad) + 0.020 * [Math]::Sin(2 * $g * $deg2rad)) % 360.0
    $e = 23.439 - 0.00000036 * $d0
    $dec = [Math]::Asin([Math]::Sin($e * $deg2rad) * [Math]::Sin($L * $deg2rad)) * $rad2deg

    $ra = [Math]::Atan2([Math]::Cos($e * $deg2rad) * [Math]::Sin($L * $deg2rad), [Math]::Cos($L * $deg2rad)) * $rad2deg
    $ra = ($ra / 15.0 + 24.0) % 24.0
    $eqT = ($q / 15.0) - $ra
    if ($eqT -gt 12.0) { $eqT -= 24.0 }
    if ($eqT -lt -12.0) { $eqT += 24.0 }

    $noon = 12.0 + $tzOffset - ($lon / 15.0) - $eqT

    $getHA = {
        param([double]$ang)
        $cosH = ([Math]::Sin(-$ang * $deg2rad) - [Math]::Sin($lat * $deg2rad) * [Math]::Sin($dec * $deg2rad)) /
                ([Math]::Cos($lat * $deg2rad) * [Math]::Cos($dec * $deg2rad))
        if ($cosH -gt 1.0) { return 0.0 }
        if ($cosH -lt -1.0) { return 12.0 }
        return ([Math]::Acos($cosH) * $rad2deg) / 15.0
    }

    $fajrHA = & $getHA $fajrAngle
    $sunHA  = & $getHA 0.833
    $magHA  = $sunHA

    $asrAngle = [Math]::Atan(1.0 / (1.0 + [Math]::Tan([Math]::Abs($lat - $dec) * $deg2rad))) * $rad2deg
    $cosAsrH = ([Math]::Sin($asrAngle * $deg2rad) - [Math]::Sin($lat * $deg2rad) * [Math]::Sin($dec * $deg2rad)) /
               ([Math]::Cos($lat * $deg2rad) * [Math]::Cos($dec * $deg2rad))
    $asrHA = if ($cosAsrH -ge -1.0 -and $cosAsrH -le 1.0) { ([Math]::Acos($cosAsrH) * $rad2deg) / 15.0 } else { 0.0 }

    $fmt = {
        param([double]$hrs)
        $hh = [Math]::Floor($hrs) % 24
        $mm = [Math]::Round(($hrs - [Math]::Floor($hrs)) * 60.0)
        if ($mm -ge 60) { $mm = 0; $hh = ($hh + 1) % 24 }
        return ("{0:D2}:{1:D2}" -f [int]$hh, [int]$mm)
    }

    $fajrStr    = & $fmt ($noon - $fajrHA)
    $sunriseStr = & $fmt ($noon - $sunHA)
    $dhuhrStr   = & $fmt ($noon)
    $asrStr     = & $fmt ($noon + $asrHA)
    $maghribStr = & $fmt ($noon + $magHA)
    $ishaStr    = if ($ishaInterval -gt 0) {
        & $fmt ($noon + $magHA + ($ishaInterval / 60.0))
    } else {
        $ishaHA = & $getHA $ishaAngle
        & $fmt ($noon + $ishaHA)
    }

    return [PSCustomObject]@{
        Fajr    = $fajrStr
        Sunrise = $sunriseStr
        Dhuhr   = $dhuhrStr
        Asr     = $asrStr
        Maghrib = $maghribStr
        Isha    = $ishaStr
    }
}

# Fetch Daily Timings with Disk Cache
function Get-DailyTimings($loc, [int]$method, [datetime]$targetDate) {
    $dateKey = $targetDate.ToString('yyyy-MM-dd')
    $cacheFile = Join-Path $script:DataDir "timings-$dateKey-m$method.json"

    if (Test-Path -LiteralPath $cacheFile) {
        try {
            $cached = Get-Content -LiteralPath $cacheFile -Raw -Encoding UTF8 | ConvertFrom-Json
            if ($cached.Fajr) { return $cached }
        } catch {}
    }

    $timings = $null
    try {
        $ts = [int64](($targetDate.ToUniversalTime() - (Get-Date '1970-01-01')).TotalSeconds)
        $url = "http://api.aladhan.com/v1/timings/$ts`?latitude=$($loc.lat)&longitude=$($loc.lon)&method=$method"
        $res = Invoke-RestMethod -Uri $url -TimeoutSec 4
        if ($res.code -eq 200 -and $res.data.timings) {
            $t = $res.data.timings
            $cleanTime = { param($val) if ($val) { ($val -replace '\s*\(.*?\)', '').Trim() } else { '00:00' } }
            $timings = [PSCustomObject]@{
                Fajr      = & $cleanTime $t.Fajr
                Sunrise   = & $cleanTime $t.Sunrise
                Dhuhr     = & $cleanTime $t.Dhuhr
                Asr       = & $cleanTime $t.Asr
                Maghrib   = & $cleanTime $t.Maghrib
                Isha      = & $cleanTime $t.Isha
                HijriDate = "$($res.data.date.hijri.day) $($res.data.date.hijri.month.ar) $($res.data.date.hijri.year)"
                Source    = 'Aladhan'
            }
        }
    } catch {}

    if (-not $timings) {
        $tzOffset = [System.TimeZoneInfo]::Local.GetUtcOffset($targetDate).TotalHours
        $offline = Calculate-OfflineTimes $loc.lat $loc.lon $tzOffset $targetDate $method
        $timings = [PSCustomObject]@{
            Fajr      = $offline.Fajr
            Sunrise   = $offline.Sunrise
            Dhuhr     = $offline.Dhuhr
            Asr       = $offline.Asr
            Maghrib   = $offline.Maghrib
            Isha      = $offline.Isha
            HijriDate = ''
            Source    = 'Offline-Calc'
        }
    }

    try {
        $json = $timings | ConvertTo-Json
        [System.IO.File]::WriteAllText($cacheFile, $json, [System.Text.Encoding]::UTF8)
    } catch {}

    return $timings
}

# Azan Alert
$script:LastAlertPrayer = ''

function Trigger-AzanAlert($prayerName) {
    if ($script:LastAlertPrayer -eq $prayerName) { return }
    $script:LastAlertPrayer = $prayerName

    $settings = Load-Settings

    # Expand the notch down into full Dynamic Island announcement
    Send-Msg @{ type = 'show' }

    if ($settings.DesktopNotify) {
        try {
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02)
            $textNodes = $template.GetElementsByTagName('text')
            $textNodes.Item(0).AppendChild($template.CreateTextNode("Azan Time: $prayerName")) | Out-Null
            $textNodes.Item(1).AppendChild($template.CreateTextNode("Time for $prayerName prayer")) | Out-Null
            $toast = [Windows.UI.Notifications.ToastNotification]::new($template)
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Nook.PrayerTimes').Show($toast)
        } catch {
            Send-Msg @{
                type  = 'notify'
                title = "Azan Time: $prayerName"
                body  = "Time for $prayerName prayer"
            }
        }
    }

    if ($settings.AzanAudio) {
        try {
            $wavCandidates = @(
                'C:\Windows\Media\Alarm01.wav',
                'C:\Windows\Media\chimes.wav',
                'C:\Windows\Media\notify.wav'
            )
            $wav = $wavCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
            if ($wav) {
                $player = New-Object System.Media.SoundPlayer $wav
                $player.Play()
            } else {
                [System.Media.SystemSounds]::Beep.Play()
            }
        } catch {}
    }
}

# Athkar State & Trigger
$script:LastAthkarTime   = [DateTime]::Now
$script:ActiveAthkar      = $null
$script:ActiveAthkarEnd   = [DateTime]::MinValue
$script:CurrentAthkarIdx  = 0
$script:TasbeehCounts     = @{}

function Trigger-AthkarAlert([int]$forceIdx = -1) {
    if ($script:AthkarList.Count -eq 0) { return }
    $settings = Load-Settings
    if (-not $settings.AthkarEnabled -and $forceIdx -lt 0) { return }

    if ($forceIdx -ge 0 -and $forceIdx -lt $script:AthkarList.Count) {
        $script:CurrentAthkarIdx = $forceIdx
    } else {
        $script:CurrentAthkarIdx = ($script:CurrentAthkarIdx + 1) % $script:AthkarList.Count
    }

    $athkar = $script:AthkarList[$script:CurrentAthkarIdx]
    $script:ActiveAthkar = $athkar
    $script:ActiveAthkarEnd = [DateTime]::Now.AddSeconds(15)

    if ($settings.AthkarToast) {
        try {
            [Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
            $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02)
            $textNodes = $template.GetElementsByTagName('text')
            $textNodes.Item(0).AppendChild($template.CreateTextNode("Athkar Reminder - Daily Remembrance")) | Out-Null
            $textNodes.Item(1).AppendChild($template.CreateTextNode("$($athkar.arabic)`n$($athkar.meaning)")) | Out-Null
            $toast = [Windows.UI.Notifications.ToastNotification]::new($template)
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Nook.PrayerTimes').Show($toast)
        } catch {
            Send-Msg @{
                type  = 'notify'
                title = "Athkar Reminder"
                body  = "$($athkar.arabic)`n$($athkar.meaning)"
            }
        }
    }

    if ($settings.AthkarAudio) {
        try {
            $wavCandidates = @(
                'C:\Windows\Media\Windows Notify System Generic.wav',
                'C:\Windows\Media\notify.wav',
                'C:\Windows\Media\chimes.wav'
            )
            $wav = $wavCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
            if ($wav) {
                $player = New-Object System.Media.SoundPlayer $wav
                $player.Play()
            }
        } catch {}
    }
}

# General UI State
$script:CurrentTab = 0
$script:TileSize   = 'wide'
$script:BadgeActive = $false

function Parse-PrayerTime([string]$hhmm, [datetime]$baseDate) {
    $parts = $hhmm.Split(':')
    $h = [int]$parts[0]
    $m = [int]$parts[1]
    return (Get-Date -Year $baseDate.Year -Month $baseDate.Month -Day $baseDate.Day -Hour $h -Minute $m -Second 0 -Millisecond 0)
}

function Format-12h([string]$hhmm) {
    if (-not $hhmm) { return '' }
    try {
        $parts = $hhmm.Split(':')
        $h = [int]$parts[0]
        $m = [int]$parts[1]
        $ampm = if ($h -ge 12) { 'PM' } else { 'AM' }
        $h12 = if ($h -eq 0) { 12 } elseif ($h -gt 12) { $h - 12 } else { $h }
        return ("{0}:{1:D2} {2}" -f $h12, $m, $ampm)
    } catch {
        return $hhmm
    }
}

# Main Prayer Schedule Evaluator
function Get-PrayerScheduleState {
    $settings = Load-Settings
    $loc = Get-UserLocation
    $now = Get-Date

    $todayTimings = Get-DailyTimings $loc $settings.CalcMethod $now
    $tommTimings  = Get-DailyTimings $loc $settings.CalcMethod ($now.AddDays(1))
    $yestTimings  = Get-DailyTimings $loc $settings.CalcMethod ($now.AddDays(-1))

    $prayers = @(
        @{ Name = 'Fajr';    TimeStr = $todayTimings.Fajr;    DT = (Parse-PrayerTime $todayTimings.Fajr $now);    IsMain = $true },
        @{ Name = 'Sunrise'; TimeStr = $todayTimings.Sunrise; DT = (Parse-PrayerTime $todayTimings.Sunrise $now); IsMain = $false },
        @{ Name = 'Dhuhr';   TimeStr = $todayTimings.Dhuhr;   DT = (Parse-PrayerTime $todayTimings.Dhuhr $now);   IsMain = $true },
        @{ Name = 'Asr';     TimeStr = $todayTimings.Asr;     DT = (Parse-PrayerTime $todayTimings.Asr $now);     IsMain = $true },
        @{ Name = 'Maghrib'; TimeStr = $todayTimings.Maghrib; DT = (Parse-PrayerTime $todayTimings.Maghrib $now); IsMain = $true },
        @{ Name = 'Isha';    TimeStr = $todayTimings.Isha;    DT = (Parse-PrayerTime $todayTimings.Isha $now);    IsMain = $true }
    )

    $next = $null
    $prev = $null

    for ($i = 0; $i -lt $prayers.Count; $i++) {
        if ($now -lt $prayers[$i].DT) {
            $next = $prayers[$i]
            if ($i -gt 0) {
                $prev = $prayers[$i - 1]
            } else {
                $prevDT = Parse-PrayerTime $yestTimings.Isha ($now.AddDays(-1))
                $prev = @{ Name = 'Isha'; DT = $prevDT; IsMain = $true }
            }
            break
        }
    }

    if (-not $next) {
        $tommFajrDT = Parse-PrayerTime $tommTimings.Fajr ($now.AddDays(1))
        $next = @{ Name = 'Fajr'; TimeStr = $tommTimings.Fajr; DT = $tommFajrDT; IsMain = $true }
        $prev = $prayers[$prayers.Count - 1]
    }

    $diffSec = ($next.DT - $now).TotalSeconds
    $totalSpanSec = [Math]::Max(60.0, ($next.DT - $prev.DT).TotalSeconds)
    $progress = [Math]::Max(0.0, [Math]::Min(1.0, 1.0 - ($diffSec / $totalSpanSec)))

    return [PSCustomObject]@{
        Now          = $now
        Next         = $next
        Prev         = $prev
        DiffSec      = $diffSec
        Progress     = $progress
        Prayers      = $prayers
        Location     = $loc
        Timings      = $todayTimings
        HijriDate    = $todayTimings.HijriDate
        Settings     = $settings
    }
}

# Build Tile View (Bento Grid)
function Build-TileView {
    $state = Get-PrayerScheduleState
    $next = $state.Next
    $diffSec = $state.DiffSec

    $hours = [int][Math]::Floor([Math]::Max(0, $diffSec) / 3600)
    $mins  = [int][Math]::Floor(([Math]::Max(0, $diffSec) % 3600) / 60)
    $secs  = [int]([Math]::Max(0, $diffSec) % 60)
    $timeCountdown = "{0:D2}:{1:D2}:{2:D2}" -f $hours, $mins, $secs

    $time12 = Format-12h $next.TimeStr
    $cityName = if ($state.Location.city) { "$($state.Location.city)" } else { "Local" }

    if ($script:TileSize -eq 'small') {
        return @{
            type = 'column'
            gap = 2
            align = 'center'
            justify = 'center'
            children = @(
                @{
                    type = 'image'
                    base64 = $script:PrayerIconBase64
                    path = $script:IconPath
                    width = 26
                    height = 26
                    rounding = 6
                },
                @{ type = 'text'; text = "$($next.Name)"; size = 'small'; bold = $true; color = 'accent' },
                @{ type = 'text'; text = "$time12"; size = 'small'; color = 'dim' }
            )
        }
    }

    $prayerPills = @()
    foreach ($p in $state.Prayers) {
        $isNext = ($p.Name -eq $next.Name)
        $p12 = Format-12h $p.TimeStr
        $prayerPills += @{
            type = 'column'
            align = 'center'
            gap = 1
            children = @(
                @{ type = 'text'; text = $p.Name; size = 'small'; bold = $isNext; color = if ($isNext) { 'accent' } else { 'dim' } },
                @{ type = 'text'; text = $p12; size = 'small'; bold = $isNext; color = if ($isNext) { 'accent' } else { 'primary' } }
            )
        }
    }

    return @{
        type = 'row'
        align = 'center'
        gap = 10
        children = @(
            @{
                type = 'image'
                base64 = $script:PrayerIconBase64
                path = $script:IconPath
                width = 38
                height = 38
                rounding = 9
            },
            @{
                type = 'column'
                gap = 2
                children = @(
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 6
                        children = @(
                            @{ type = 'text'; text = "Next: $($next.Name)"; size = 'body'; bold = $true },
                            @{ type = 'text'; text = "$time12"; size = 'small'; color = 'accent'; bold = $true }
                        )
                    },
                    @{
                        type = 'text'
                        text = "$timeCountdown left - $cityName"
                        size = 'small'
                        color = 'dim'
                    }
                )
            },
            @{ type = 'spacer' },
            @{
                type = 'row'
                align = 'center'
                gap = 8
                children = $prayerPills
            }
        )
    }
}

# Build Page View (Interactive Settings, Athkar and Schedule)
function Build-PageView {
    $state = Get-PrayerScheduleState
    $settings = $state.Settings
    $next = $state.Next
    $diffSec = $state.DiffSec

    $hours = [int][Math]::Floor([Math]::Max(0, $diffSec) / 3600)
    $mins  = [int][Math]::Floor(([Math]::Max(0, $diffSec) % 3600) / 60)
    $secs  = [int]([Math]::Max(0, $diffSec) % 60)
    $countdownStr = "{0:D2}:{1:D2}:{2:D2}" -f $hours, $mins, $secs

    $headerCard = @{
        type = 'card'
        padding = 10
        children = @(
            @{
                type = 'row'
                align = 'center'
                gap = 12
                children = @(
                    @{
                        type = 'image'
                        base64 = $script:PrayerIconBase64
                        path = $script:IconPath
                        width = 44
                        height = 44
                        rounding = 11
                    },
                    @{
                        type = 'column'
                        gap = 2
                        children = @(
                            @{ type = 'text'; text = "Prayer Times - Next: $($next.Name) at $(Format-12h $next.TimeStr)"; size = 'body'; bold = $true },
                            @{
                                type = 'row'
                                align = 'center'
                                gap = 6
                                children = @(
                                    @{ type = 'text'; text = "Remaining: $countdownStr"; size = 'small'; bold = $true; color = 'accent' },
                                    @{ type = 'text'; text = "- $($state.Location.city), $($state.Location.country)"; size = 'small'; color = 'dim' }
                                )
                            }
                        )
                    },
                    @{ type = 'spacer' },
                    @{
                        type = 'button'
                        id = 'btn_refresh_loc'
                        label = 'Refresh'
                        icon = 'arrows-clockwise'
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
                options = @("Today's Timetable", "Athkar & Tasbeeh", "Settings & Alerts")
                selected = [int]$script:CurrentTab
            }
        )
    }

    $tabContent = @()

    if ($script:CurrentTab -eq 0) {
        $scheduleRows = @()
        foreach ($p in $state.Prayers) {
            $isNext = ($p.Name -eq $next.Name)
            $scheduleRows += @{
                type = 'row'
                align = 'center'
                children = @(
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 8
                        children = @(
                            @{ type = 'icon'; icon = if ($isNext) { 'check-circle' } else { 'clock' }; size = 16; color = if ($isNext) { 'accent' } else { 'dim' } },
                            @{ type = 'text'; text = "$($p.Name)"; size = 'body'; bold = $isNext; color = if ($isNext) { 'accent' } else { 'primary' } }
                        )
                    },
                    @{ type = 'spacer' },
                    @{ type = 'text'; text = (Format-12h $p.TimeStr); size = 'body'; bold = $isNext; color = if ($isNext) { 'accent' } else { 'primary' } },
                    @{ type = 'text'; text = "($($p.TimeStr))"; size = 'small'; color = 'dim' }
                )
            }
            $scheduleRows += @{ type = 'divider' }
        }

        if ($scheduleRows.Count -gt 0) {
            $scheduleRows = $scheduleRows[0..($scheduleRows.Count - 2)]
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 6
            children = $scheduleRows
        }
    }
    elseif ($script:CurrentTab -eq 1) {
        $curr = if ($script:AthkarList.Count -gt 0) {
            $script:AthkarList[$script:CurrentAthkarIdx % $script:AthkarList.Count]
        } else { $null }

        $aid = if ($curr) { [string]$curr.id } else { 'default' }
        $count = if ($script:TasbeehCounts.ContainsKey($aid)) { [int]$script:TasbeehCounts[$aid] } else { 0 }
        $target = if ($curr -and $curr.target) { [int]$curr.target } else { 33 }

        if ($curr) {
            $tabContent += @{
                type = 'card'
                padding = 12
                gap = 8
                children = @(
                    @{
                        type = 'row'
                        align = 'center'
                        children = @(
                            @{ type = 'text'; text = 'Active Remembrance (Dhikr)'; size = 'small'; bold = $true; color = 'accent' },
                            @{ type = 'spacer' },
                            @{ type = 'text'; text = "$($script:CurrentAthkarIdx + 1) / $($script:AthkarList.Count)"; size = 'small'; color = 'dim' }
                        )
                    },
                    @{
                        type = 'card'
                        padding = 10
                        gap = 6
                        children = @(
                            @{ type = 'text'; text = $curr.arabic; size = 'title'; bold = $true; color = 'accent' },
                            @{ type = 'text'; text = "$($curr.translit) - $($curr.meaning)"; size = 'small'; color = 'primary' },
                            @{ type = 'text'; text = "Virtue: $($curr.virtue)"; size = 'small'; color = 'dim' }
                        )
                    },
                    @{
                        type = 'row'
                        align = 'center'
                        gap = 8
                        children = @(
                            @{
                                type = 'button'
                                id = 'btn_tasbeeh_count'
                                label = "Recite & Count: $count / $target"
                                icon = 'fingerprint'
                            },
                            @{
                                type = 'button'
                                id = 'btn_tasbeeh_reset'
                                icon = 'arrow-counter-clockwise'
                                tooltip = 'Reset counter'
                            },
                            @{ type = 'spacer' },
                            @{
                                type = 'button'
                                id = 'btn_athkar_prev'
                                icon = 'caret-left'
                                tooltip = 'Previous'
                            },
                            @{
                                type = 'button'
                                id = 'btn_athkar_next'
                                icon = 'caret-right'
                                tooltip = 'Next'
                            },
                            @{
                                type = 'button'
                                id = 'btn_send_notch'
                                label = 'Show on Notch'
                                icon = 'sparkle'
                            }
                        )
                    }
                )
            }
        }

        # List of all Adhkar
        $athkarRows = @()
        for ($i = 0; $i -lt $script:AthkarList.Count; $i++) {
            $item = $script:AthkarList[$i]
            $isSelected = ($i -eq $script:CurrentAthkarIdx)
            $athkarRows += @{
                type = 'row'
                align = 'center'
                gap = 8
                children = @(
                    @{
                        type = 'column'
                        gap = 1
                        children = @(
                            @{ type = 'text'; text = $item.short_ar; size = 'body'; bold = $isSelected; color = if ($isSelected) { 'accent' } else { 'primary' } },
                            @{ type = 'text'; text = $item.translit; size = 'small'; color = 'dim' }
                        )
                    },
                    @{ type = 'spacer' },
                    @{
                        type = 'button'
                        id = "select_athkar_$i"
                        label = if ($isSelected) { 'Selected' } else { 'Select' }
                        icon = if ($isSelected) { 'check' } else { 'book-open' }
                    }
                )
            }
            $athkarRows += @{ type = 'divider' }
        }
        if ($athkarRows.Count -gt 0) {
            $athkarRows = $athkarRows[0..($athkarRows.Count - 2)]
        }

        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 6
            children = $athkarRows
        }
    }
    else {
        $threshIdx = [array]::IndexOf($script:ThresholdValues, [int]$settings.ShowBeforeMinutes)
        if ($threshIdx -lt 0) { $threshIdx = 0 }

        $methodIdx = [array]::IndexOf($script:MethodIds, [int]$settings.CalcMethod)
        if ($methodIdx -lt 0) { $methodIdx = 0 }

        $athkarIntIdx = [array]::IndexOf($script:AthkarIntervalValues, [int]$settings.AthkarIntervalMin)
        if ($athkarIntIdx -lt 0) { $athkarIntIdx = 3 }

        $athkarLangIdx = [array]::IndexOf($script:AthkarLangValues, [int]$settings.AthkarLanguage)
        if ($athkarLangIdx -lt 0) { $athkarLangIdx = 0 }

        # Card 1: Notch Ring Timing Setting
        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Notch Countdown Ring (Dynamic Island)'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'column'
                    gap = 2
                    children = @(
                        @{ type = 'text'; text = 'Show Countdown on Notch Before Azan:'; size = 'body' },
                        @{ type = 'text'; text = 'Controls when the timer and ring appear on the top collapsed bar. Outside this duration, Nook displays its normal clock.'; size = 'small'; color = 'dim' }
                    )
                },
                @{
                    type = 'chips'
                    id = 'chip_threshold'
                    options = $script:ThresholdOptions
                    selected = [int]$threshIdx
                }
            )
        }

        # Card 2: Athkar Notifications in Outer Notch
        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Athkar Remembrances in Outer Notch'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Enable Periodic Athkar in Notch'; size = 'body' },
                                @{ type = 'text'; text = 'Displays authentic Islamic remembrances in the outer notch at chosen intervals'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_athkar_enabled'
                            value = [bool]$settings.AthkarEnabled
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'column'
                    gap = 4
                    children = @(
                        @{ type = 'text'; text = 'Remembrance Interval (Every how many minutes):'; size = 'body' },
                        @{
                            type = 'chips'
                            id = 'chip_athkar_interval'
                            options = $script:AthkarIntervalOptions
                            selected = [int]$athkarIntIdx
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'column'
                    gap = 4
                    children = @(
                        @{ type = 'text'; text = 'Notch Outer Display Style:'; size = 'body' },
                        @{
                            type = 'chips'
                            id = 'chip_athkar_lang'
                            options = $script:AthkarLangOptions
                            selected = [int]$athkarLangIdx
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
                                @{ type = 'text'; text = 'Desktop Notification Popup'; size = 'body' },
                                @{ type = 'text'; text = 'Shows a Windows notification popup with virtue and meaning'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_athkar_toast'
                            value = [bool]$settings.AthkarToast
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
                                @{ type = 'text'; text = 'Play Gentle Chime on Arrival'; size = 'body' },
                                @{ type = 'text'; text = 'Plays a quiet, soothing chime when a remembrance arrives'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_athkar_audio'
                            value = [bool]$settings.AthkarAudio
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'text'; text = 'Test Athkar Notification:'; size = 'body' },
                        @{ type = 'spacer' },
                        @{
                            type = 'button'
                            id = 'btn_test_athkar'
                            label = 'Test on Notch Now'
                            icon = 'sparkle'
                        }
                    )
                }
            )
        }

        # Card 3: Location & Calculation Method
        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Calculation Method & Location'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Auto-Detect Location (IP Geolocation)'; size = 'body' },
                                @{ type = 'text'; text = "Current: $($state.Location.city), $($state.Location.country) (Lat: $($state.Location.lat), Lon: $($state.Location.lon))"; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_auto_loc'
                            value = [bool]$settings.AutoLocation
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'column'
                    gap = 4
                    children = @(
                        @{ type = 'text'; text = 'Islamic Calculation Authority:'; size = 'body' },
                        @{
                            type = 'chips'
                            id = 'chip_calc_method'
                            options = $script:MethodLabels
                            selected = [int]$methodIdx
                        }
                    )
                }
            )
        }

        # Card 4: Azan Sound & Notifications
        $tabContent += @{
            type = 'card'
            padding = 10
            gap = 8
            children = @(
                @{ type = 'text'; text = 'Azan Alerts'; size = 'small'; bold = $true; color = 'accent' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{
                            type = 'column'
                            gap = 1
                            children = @(
                                @{ type = 'text'; text = 'Play Chime / Audio Alert at Azan'; size = 'body' },
                                @{ type = 'text'; text = 'Plays a gentle chime when the prayer time arrives'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_sound'
                            value = [bool]$settings.AzanAudio
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
                                @{ type = 'text'; text = 'Desktop Notification Popup'; size = 'body' },
                                @{ type = 'text'; text = 'Shows a Windows notification popup when Azan arrives'; size = 'small'; color = 'dim' }
                            )
                        },
                        @{ type = 'spacer' },
                        @{
                            type = 'toggle'
                            id = 'toggle_notify'
                            value = [bool]$settings.DesktopNotify
                        }
                    )
                },
                @{ type = 'divider' },
                @{
                    type = 'row'
                    align = 'center'
                    children = @(
                        @{ type = 'text'; text = 'Test Azan Alert Now:'; size = 'body' },
                        @{ type = 'spacer' },
                        @{
                            type = 'button'
                            id = 'btn_test_alert'
                            label = 'Test Audio & Notification'
                            icon = 'bell-ringing'
                        }
                    )
                }
            )
        }
    }

    $allChildren = @($headerCard, $tabsRow) + $tabContent

    return @{
        type = 'column'
        gap = 8
        scroll = $true
        children = $allChildren
    }
}

function Update-NotchBadge {
    $now = Get-Date
    $settings = Load-Settings

    # 1. Athkar Notification Override on Outer Notch
    if ($script:ActiveAthkar -and $now -lt $script:ActiveAthkarEnd) {
        $remSec = ($script:ActiveAthkarEnd - $now).TotalSeconds
        $prog = [Math]::Max(0.0, [Math]::Min(1.0, $remSec / 15.0))
        $lang = [int]$settings.AthkarLanguage
        $lbl = switch ($lang) {
            0 { "$($script:ActiveAthkar.short_ar)" }
            2 { "Dhikr: $($script:ActiveAthkar.translit)" }
            default { "$($script:ActiveAthkar.short_ar)" }
        }

        Send-Msg @{
            type = 'badge'
            badge = @{
                kind = 'ring'
                progress = [float]$prog
                label = $lbl
            }
        }
        $script:BadgeActive = $true
        return
    } else {
        $script:ActiveAthkar = $null
    }

    # 2. Regular Prayer Times Countdown Ring
    $state = Get-PrayerScheduleState
    $diffSec = $state.DiffSec
    $next = $state.Next

    $threshMin = [int]$settings.ShowBeforeMinutes
    $diffMin = $diffSec / 60.0

    if ($threshMin -lt 0) { $threshMin = 30 }

    $shouldShow = ($threshMin -eq 0) -or ($diffMin -le $threshMin -and $diffSec -ge -120)

    if ($shouldShow) {
        if ($diffSec -le 0 -and $diffSec -ge -15) {
            Trigger-AzanAlert $next.Name
        }

        $prog = if ($threshMin -gt 0) {
            [Math]::Max(0.0, [Math]::Min(1.0, 1.0 - ($diffSec / ($threshMin * 60.0))))
        } else {
            $state.Progress
        }

        $lbl = ""
        if ($diffSec -le 0 -and $diffSec -ge -120) {
            $lbl = "Azan $($next.Name)"
            $prog = 1.0
        } else {
            $hours = [int][Math]::Floor($diffSec / 3600)
            $mins  = [int][Math]::Floor(($diffSec % 3600) / 60)
            if ($hours -gt 0) {
                if ($mins -gt 0) {
                    $lbl = "$($next.Name) ${hours}h${mins}m"
                } else {
                    $lbl = "$($next.Name) ${hours}h"
                }
            } else {
                $lbl = "$($next.Name) ${mins}m"
            }
        }

        Send-Msg @{
            type = 'badge'
            badge = @{
                kind = 'ring'
                progress = [float]$prog
                label = $lbl
            }
        }
        $script:BadgeActive = $true
    } else {
        if ($script:BadgeActive) {
            Send-Msg @{
                type = 'badge'
                badge = $null
            }
            $script:BadgeActive = $false
        }
    }
}

function Update-Views {
    Send-Msg @{
        type = 'tile'
        view = (Build-TileView)
    }
    Send-Msg @{
        type = 'page'
        view = (Build-PageView)
    }
}

# Initial Startup
Update-Views
Update-NotchBadge

# Main Event & Periodic Loop
$script:PageIsOpen = $false
$script:LastTileSecond = [DateTime]::MinValue
$script:LastBadgeSecond = [DateTime]::MinValue

while ($true) {
    while ($null -ne ($line = [PrayerStdinReader]::TryReadLine())) {
        if ([string]::IsNullOrWhiteSpace($line)) { continue }
        try {
            $msg = $line | ConvertFrom-Json
        } catch { continue }

        switch ($msg.type) {
            'page_open' {
                $script:PageIsOpen = $true
                Update-Views
            }
            'page_close' {
                $script:PageIsOpen = $false
            }
            'layout' {
                if ($msg.target -eq 'tile' -and $msg.size) {
                    $script:TileSize = [string]$msg.size
                    Send-Msg @{ type = 'tile'; view = (Build-TileView) }
                }
            }
            'event' {
                $id = [string]$msg.id
                $val = $msg.value
                $settings = Load-Settings

                if ($id -eq 'nav_tab') {
                    $script:CurrentTab = Get-IndexValue $val
                    Update-Views
                }
                elseif ($id -eq 'chip_threshold') {
                    $idx = Get-IndexValue $val
                    if ($idx -ge 0 -and $idx -lt $script:ThresholdValues.Count) {
                        $settings.ShowBeforeMinutes = $script:ThresholdValues[$idx]
                        Save-Settings $settings
                        Update-Views
                        Update-NotchBadge
                    }
                }
                elseif ($id -eq 'chip_calc_method') {
                    $idx = Get-IndexValue $val
                    if ($idx -ge 0 -and $idx -lt $script:MethodIds.Count) {
                        $settings.CalcMethod = $script:MethodIds[$idx]
                        Save-Settings $settings
                        Update-Views
                        Update-NotchBadge
                    }
                }
                elseif ($id -eq 'toggle_auto_loc') {
                    $settings.AutoLocation = [bool]$val
                    Save-Settings $settings
                    Update-Views
                }
                elseif ($id -eq 'toggle_sound') {
                    $settings.AzanAudio = [bool]$val
                    Save-Settings $settings
                    Update-Views
                }
                elseif ($id -eq 'toggle_notify') {
                    $settings.DesktopNotify = [bool]$val
                    Save-Settings $settings
                    Update-Views
                }
                elseif ($id -eq 'btn_refresh_loc') {
                    $null = Get-UserLocation $true
                    Update-Views
                    Update-NotchBadge
                }
                elseif ($id -eq 'btn_test_alert') {
                    Trigger-AzanAlert 'Test'
                    $script:LastAlertPrayer = ''
                }
                elseif ($id -eq 'toggle_athkar_enabled') {
                    $settings.AthkarEnabled = [bool]$val
                    Save-Settings $settings
                    Update-Views
                    Update-NotchBadge
                }
                elseif ($id -eq 'chip_athkar_interval') {
                    $idx = Get-IndexValue $val
                    if ($idx -ge 0 -and $idx -lt $script:AthkarIntervalValues.Count) {
                        $settings.AthkarIntervalMin = $script:AthkarIntervalValues[$idx]
                        Save-Settings $settings
                        Update-Views
                    }
                }
                elseif ($id -eq 'chip_athkar_lang') {
                    $idx = Get-IndexValue $val
                    if ($idx -ge 0 -and $idx -lt $script:AthkarLangValues.Count) {
                        $settings.AthkarLanguage = $script:AthkarLangValues[$idx]
                        Save-Settings $settings
                        Update-Views
                        Update-NotchBadge
                    }
                }
                elseif ($id -eq 'toggle_athkar_toast') {
                    $settings.AthkarToast = [bool]$val
                    Save-Settings $settings
                    Update-Views
                }
                elseif ($id -eq 'toggle_athkar_audio') {
                    $settings.AthkarAudio = [bool]$val
                    Save-Settings $settings
                    Update-Views
                }
                elseif ($id -eq 'btn_test_athkar' -or $id -eq 'btn_send_notch') {
                    Trigger-AthkarAlert $script:CurrentAthkarIdx
                    Update-NotchBadge
                }
                elseif ($id -eq 'btn_tasbeeh_count') {
                    if ($script:AthkarList.Count -gt 0) {
                        $aid = [string]$script:AthkarList[$script:CurrentAthkarIdx].id
                        if (-not $script:TasbeehCounts.ContainsKey($aid)) { $script:TasbeehCounts[$aid] = 0 }
                        $script:TasbeehCounts[$aid]++
                        Update-Views
                    }
                }
                elseif ($id -eq 'btn_tasbeeh_reset') {
                    if ($script:AthkarList.Count -gt 0) {
                        $aid = [string]$script:AthkarList[$script:CurrentAthkarIdx].id
                        $script:TasbeehCounts[$aid] = 0
                        Update-Views
                    }
                }
                elseif ($id -eq 'btn_athkar_prev') {
                    $count = $script:AthkarList.Count
                    if ($count -gt 0) {
                        $script:CurrentAthkarIdx = ($script:CurrentAthkarIdx - 1 + $count) % $count
                        Update-Views
                    }
                }
                elseif ($id -eq 'btn_athkar_next') {
                    $count = $script:AthkarList.Count
                    if ($count -gt 0) {
                        $script:CurrentAthkarIdx = ($script:CurrentAthkarIdx + 1) % $count
                        Update-Views
                    }
                }
                elseif ($id.StartsWith('select_athkar_')) {
                    $idxStr = $id.Substring('select_athkar_'.Length)
                    try {
                        $targetIdx = [int]$idxStr
                        if ($targetIdx -ge 0 -and $targetIdx -lt $script:AthkarList.Count) {
                            $script:CurrentAthkarIdx = $targetIdx
                            Update-Views
                        }
                    } catch {}
                }
            }
            'shutdown' {
                exit 0
            }
        }
    }

    $now = Get-Date

    # Periodic Athkar check
    $settings = Load-Settings
    if ($settings.AthkarEnabled -and $script:AthkarList.Count -gt 0) {
        $intervalMin = [int]$settings.AthkarIntervalMin
        if ($intervalMin -le 0) { $intervalMin = 30 }
        if (($now - $script:LastAthkarTime).TotalMinutes -ge $intervalMin) {
            $script:LastAthkarTime = $now
            Trigger-AthkarAlert
        }
    }

    if (($now - $script:LastBadgeSecond).TotalSeconds -ge 1) {
        $script:LastBadgeSecond = $now
        Update-NotchBadge
    }

    if ($script:PageIsOpen) {
        Send-Msg @{ type = 'page'; view = (Build-PageView) }
    }

    if (($now - $script:LastTileSecond).TotalSeconds -ge 5) {
        $script:LastTileSecond = $now
        Send-Msg @{ type = 'tile'; view = (Build-TileView) }
    }

    Start-Sleep -Milliseconds 800
}
