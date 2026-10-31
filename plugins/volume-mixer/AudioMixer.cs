// Core Audio per-application volume mixer + icon extractor for Nook.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;

public class AppAudioInfo
{
    public string Key;
    public string Name;
    public string ExePath;
    public int Volume;      // 0..100
    public bool Muted;
    public bool Active;     // true if actively playing audio right now
}

public static class AudioMixer
{
    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
    class MMDeviceEnumeratorComObject { }

    [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDeviceEnumerator
    {
        int EnumAudioEndpoints(int dataFlow, int dwStateMask, out IntPtr ppDevices);
        int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice ppEndpoint);
    }

    [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDevice
    {
        int Activate(ref Guid iid, int dwClsCtx, IntPtr pActivationParams, [MarshalAs(UnmanagedType.IUnknown)] out object ppInterface);
    }

    [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioEndpointVolume
    {
        int RegisterControlChangeNotify(IntPtr pNotify);
        int UnregisterControlChangeNotify(IntPtr pNotify);
        int GetChannelCount(out uint pnChannelCount);
        int SetMasterVolumeLevel(float fLevelDB, ref Guid pguidEventContext);
        int SetMasterVolumeLevelScalar(float fLevel, ref Guid pguidEventContext);
        int GetMasterVolumeLevel(out float pfLevelDB);
        int GetMasterVolumeLevelScalar(out float pfLevel);
        int SetChannelVolumeLevel(uint nChannel, float fLevelDB, ref Guid pguidEventContext);
        int SetChannelVolumeLevelScalar(uint nChannel, float fLevel, ref Guid pguidEventContext);
        int GetChannelVolumeLevel(uint nChannel, out float pfLevelDB);
        int GetChannelVolumeLevelScalar(uint nChannel, out float pfLevel);
        int SetMute([MarshalAs(UnmanagedType.Bool)] bool bMute, ref Guid pguidEventContext);
        int GetMute([MarshalAs(UnmanagedType.Bool)] out bool pbMute);
    }

    [Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionManager2
    {
        int GetAudioSessionControl(IntPtr AudioSessionGuid, uint StreamFlags, out IntPtr SessionControl);
        int GetSimpleAudioVolume(IntPtr AudioSessionGuid, uint StreamFlags, out IntPtr AudioVolume);
        int GetSessionEnumerator(out IAudioSessionEnumerator SessionEnum);
    }

    [Guid("E2F5BB11-0570-40CA-ACDD-3AA01277DEE8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionEnumerator
    {
        int GetCount(out int SessionCount);
        int GetSession(int SessionCount, out IAudioSessionControl2 Session);
    }

    [Guid("bfb7ff88-7239-4fc9-8fa2-07c950be9c6d"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionControl2
    {
        int GetState(out int pRetVal);
        int GetDisplayName([MarshalAs(UnmanagedType.LPWStr)] out string pRetVal);
        int SetDisplayName([MarshalAs(UnmanagedType.LPWStr)] string Value, ref Guid EventContext);
        int GetIconPath([MarshalAs(UnmanagedType.LPWStr)] out string pRetVal);
        int SetIconPath([MarshalAs(UnmanagedType.LPWStr)] string Value, ref Guid EventContext);
        int GetGroupingParam(out Guid pRetVal);
        int SetGroupingParam(ref Guid Override, ref Guid EventContext);
        int RegisterAudioSessionNotification(IntPtr NewNotifications);
        int UnregisterAudioSessionNotification(IntPtr NewNotifications);
        int GetSessionIdentifier([MarshalAs(UnmanagedType.LPWStr)] out string pRetVal);
        int GetSessionInstanceIdentifier([MarshalAs(UnmanagedType.LPWStr)] out string pRetVal);
        int GetProcessId(out uint pRetVal);
        int IsSystemSoundsSession();
        int SetDuckingPreference(bool optOut);
    }

    [Guid("87CE5498-68D6-44E5-9215-6DA47EF883D8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface ISimpleAudioVolume
    {
        int SetMasterVolume(float fLevel, ref Guid EventContext);
        int GetMasterVolume(out float pfLevel);
        int SetMute([MarshalAs(UnmanagedType.Bool)] bool bMute, ref Guid EventContext);
        int GetMute([MarshalAs(UnmanagedType.Bool)] out bool pbMute);
    }

    static IMMDevice GetDefaultSpeaker()
    {
        var enumObj = (IMMDeviceEnumerator)(new MMDeviceEnumeratorComObject());
        IMMDevice dev;
        enumObj.GetDefaultAudioEndpoint(0, 1, out dev);
        return dev;
    }

    static IAudioEndpointVolume GetEndpointVolume()
    {
        var dev = GetDefaultSpeaker();
        Guid iid = new Guid("5CDF2C82-841E-4546-9722-0CF74078229A");
        object obj;
        dev.Activate(ref iid, 23, IntPtr.Zero, out obj);
        return (IAudioEndpointVolume)obj;
    }

    public static int GetMasterVolume(out bool muted)
    {
        muted = false;
        try
        {
            var ep = GetEndpointVolume();
            float lvl; ep.GetMasterVolumeLevelScalar(out lvl);
            ep.GetMute(out muted);
            return (int)Math.Round(lvl * 100f);
        }
        catch { return 100; }
    }

    public static void SetMasterVolume(int pct)
    {
        try
        {
            var ep = GetEndpointVolume();
            Guid ctx = Guid.Empty;
            float lvl = Math.Max(0f, Math.Min(1f, pct / 100f));
            ep.SetMasterVolumeLevelScalar(lvl, ref ctx);
            if (pct > 0) ep.SetMute(false, ref ctx);
        }
        catch { }
    }

    public static void ToggleMasterMute()
    {
        try
        {
            var ep = GetEndpointVolume();
            bool m; ep.GetMute(out m);
            Guid ctx = Guid.Empty;
            ep.SetMute(!m, ref ctx);
        }
        catch { }
    }

    static string NormalizeKey(uint pid, bool isSystem, out string friendlyName, out string exePath)
    {
        friendlyName = "System Sounds";
        exePath = "";
        if (isSystem || pid == 0)
        {
            exePath = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.System), "sndvol.exe");
            return "system";
        }
        try
        {
            var p = Process.GetProcessById((int)pid);
            string pname = p.ProcessName;
            try
            {
                exePath = p.MainModule.FileName;
                var vi = p.MainModule.FileVersionInfo;
                if (!string.IsNullOrWhiteSpace(vi.FileDescription) && vi.FileDescription.Length < 40)
                    friendlyName = vi.FileDescription.Trim();
                else if (!string.IsNullOrWhiteSpace(vi.ProductName) && vi.ProductName.Length < 40)
                    friendlyName = vi.ProductName.Trim();
                else
                    friendlyName = pname;
            }
            catch
            {
                friendlyName = pname;
            }

            string low = pname.ToLowerInvariant();
            if (low == "steamwebhelper") { friendlyName = "Steam"; return "steam"; }
            if (low == "discord") { friendlyName = "Discord"; return "discord"; }
            if (low == "chrome") { friendlyName = "Google Chrome"; return "chrome"; }
            if (low == "msedge") { friendlyName = "Microsoft Edge"; return "msedge"; }
            if (low == "firefox") { friendlyName = "Firefox"; return "firefox"; }
            if (low == "spotify") { friendlyName = "Spotify"; return "spotify"; }
            return low;
        }
        catch
        {
            return null;
        }
    }

    public static List<AppAudioInfo> GetApps()
    {
        var map = new Dictionary<string, AppAudioInfo>();
        try
        {
            var dev = GetDefaultSpeaker();
            Guid iidMgr = new Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F");
            object mgrObj;
            dev.Activate(ref iidMgr, 23, IntPtr.Zero, out mgrObj);
            var mgr = (IAudioSessionManager2)mgrObj;
            IAudioSessionEnumerator sesEnum;
            mgr.GetSessionEnumerator(out sesEnum);
            int count;
            sesEnum.GetCount(out count);

            for (int i = 0; i < count; i++)
            {
                IAudioSessionControl2 ctl;
                sesEnum.GetSession(i, out ctl);
                int state; ctl.GetState(out state);
                if (state == 2) continue; // expired

                uint pid; ctl.GetProcessId(out pid);
                string disp; ctl.GetDisplayName(out disp);
                bool isSys = (pid == 0) || (!string.IsNullOrEmpty(disp) && disp.IndexOf("AudioSrv", StringComparison.OrdinalIgnoreCase) >= 0);
                string name, exe;
                string key = NormalizeKey(pid, isSys, out name, out exe);
                if (string.IsNullOrEmpty(key)) continue;

                var vol = (ISimpleAudioVolume)ctl;
                float level; vol.GetMasterVolume(out level);
                bool mute; vol.GetMute(out mute);
                int pct = (int)Math.Round(level * 100f);

                AppAudioInfo existing;
                if (!map.TryGetValue(key, out existing))
                {
                    map[key] = new AppAudioInfo
                    {
                        Key = key,
                        Name = name,
                        ExePath = exe,
                        Volume = pct,
                        Muted = mute,
                        Active = (state == 1)
                    };
                }
                else
                {
                    if (state == 1 && !existing.Active)
                    {
                        existing.Volume = pct;
                        existing.Muted = mute;
                        existing.Active = true;
                    }
                    if (string.IsNullOrEmpty(existing.ExePath) && !string.IsNullOrEmpty(exe))
                        existing.ExePath = exe;
                }
            }
        }
        catch { }

        var list = new List<AppAudioInfo>(map.Values);
        list.Sort((a, b) =>
        {
            if (a.Key == "system") return 1;
            if (b.Key == "system") return -1;
            if (a.Active != b.Active) return a.Active ? -1 : 1;
            return string.Compare(a.Name, b.Name, StringComparison.OrdinalIgnoreCase);
        });
        return list;
    }

    public static void SetAppVolume(string targetKey, int pct)
    {
        try
        {
            var dev = GetDefaultSpeaker();
            Guid iidMgr = new Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F");
            object mgrObj;
            dev.Activate(ref iidMgr, 23, IntPtr.Zero, out mgrObj);
            var mgr = (IAudioSessionManager2)mgrObj;
            IAudioSessionEnumerator sesEnum;
            mgr.GetSessionEnumerator(out sesEnum);
            int count;
            sesEnum.GetCount(out count);
            float lvl = Math.Max(0f, Math.Min(1f, pct / 100f));
            Guid ctx = Guid.Empty;

            for (int i = 0; i < count; i++)
            {
                IAudioSessionControl2 ctl;
                sesEnum.GetSession(i, out ctl);
                uint pid; ctl.GetProcessId(out pid);
                string disp; ctl.GetDisplayName(out disp);
                bool isSys = (pid == 0) || (!string.IsNullOrEmpty(disp) && disp.IndexOf("AudioSrv", StringComparison.OrdinalIgnoreCase) >= 0);
                string name, exe;
                string key = NormalizeKey(pid, isSys, out name, out exe);
                if (key == targetKey)
                {
                    var vol = (ISimpleAudioVolume)ctl;
                    vol.SetMasterVolume(lvl, ref ctx);
                    if (pct > 0) vol.SetMute(false, ref ctx);
                }
            }
        }
        catch { }
    }

    public static void ToggleAppMute(string targetKey)
    {
        try
        {
            var dev = GetDefaultSpeaker();
            Guid iidMgr = new Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F");
            object mgrObj;
            dev.Activate(ref iidMgr, 23, IntPtr.Zero, out mgrObj);
            var mgr = (IAudioSessionManager2)mgrObj;
            IAudioSessionEnumerator sesEnum;
            mgr.GetSessionEnumerator(out sesEnum);
            int count;
            sesEnum.GetCount(out count);
            Guid ctx = Guid.Empty;
            bool? targetMute = null;

            for (int i = 0; i < count; i++)
            {
                IAudioSessionControl2 ctl;
                sesEnum.GetSession(i, out ctl);
                uint pid; ctl.GetProcessId(out pid);
                string disp; ctl.GetDisplayName(out disp);
                bool isSys = (pid == 0) || (!string.IsNullOrEmpty(disp) && disp.IndexOf("AudioSrv", StringComparison.OrdinalIgnoreCase) >= 0);
                string name, exe;
                string key = NormalizeKey(pid, isSys, out name, out exe);
                if (key == targetKey)
                {
                    var vol = (ISimpleAudioVolume)ctl;
                    if (!targetMute.HasValue)
                    {
                        bool cur; vol.GetMute(out cur);
                        targetMute = !cur;
                    }
                    vol.SetMute(targetMute.Value, ref ctx);
                }
            }
        }
        catch { }
    }

    // Simple icon extractor for app .exe icons
    public static void SaveExeIcon(string exePath, string pngPath)
    {
        if (string.IsNullOrEmpty(exePath) || !File.Exists(exePath) || File.Exists(pngPath)) return;
        try
        {
            using (var ico = Icon.ExtractAssociatedIcon(exePath))
            {
                if (ico == null) return;
                using (var src = ico.ToBitmap())
                using (var dst = new Bitmap(64, 64, PixelFormat.Format32bppArgb))
                {
                    using (var g = Graphics.FromImage(dst))
                    {
                        g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                        g.SmoothingMode = SmoothingMode.HighQuality;
                        g.Clear(Color.Transparent);
                        g.DrawImage(src, new Rectangle(0, 0, 64, 64));
                    }
                    dst.Save(pngPath, ImageFormat.Png);
                }
            }
        }
        catch { }
    }
}
