// Icon extractor, file picker, and mouse hit-tester for the Nook "Game Launcher" plugin.
using System;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;
using System.Threading;
using System.Windows.Forms;

public static class NookIcon
{
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    static extern uint PrivateExtractIcons(string file, int index, int cx, int cy,
        IntPtr[] icons, uint[] ids, uint count, uint flags);

    [DllImport("user32.dll")] static extern bool DestroyIcon(IntPtr h);
    [DllImport("user32.dll")] static extern bool GetIconInfo(IntPtr h, out ICONINFO info);
    [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr hwnd);
    [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);
    [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr h);
    [DllImport("gdi32.dll")] static extern int GetObject(IntPtr h, int size, ref BITMAP bmp);
    [DllImport("gdi32.dll")] static extern int GetDIBits(IntPtr dc, IntPtr hbm, uint start,
        uint lines, byte[] bits, ref BITMAPINFOHEADER info, uint usage);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    static extern IntPtr SHGetFileInfo(string path, uint attrs, ref SHFILEINFO info,
        uint size, uint flags);

    const uint SHGFI_ICON = 0x100;
    const uint SHGFI_LARGEICON = 0x0;

    [StructLayout(LayoutKind.Sequential)]
    struct ICONINFO { public bool fIcon; public int xHotspot; public int yHotspot; public IntPtr hbmMask; public IntPtr hbmColor; }

    [StructLayout(LayoutKind.Sequential)]
    struct BITMAP { public int bmType; public int bmWidth; public int bmHeight; public int bmWidthBytes; public ushort bmPlanes; public ushort bmBitsPixel; public IntPtr bmBits; }

    [StructLayout(LayoutKind.Sequential)]
    struct BITMAPINFOHEADER
    {
        public uint biSize; public int biWidth; public int biHeight; public ushort biPlanes; public ushort biBitCount;
        public uint biCompression; public uint biSizeImage; public int biXPelsPerMeter; public int biYPelsPerMeter;
        public uint biClrUsed; public uint biClrImportant;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    struct SHFILEINFO
    {
        public IntPtr hIcon; public int iIcon; public uint dwAttributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string szDisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)] public string szTypeName;
    }

    static GraphicsPath RoundedRect(RectangleF bounds, float radius)
    {
        float d = radius * 2f;
        var path = new GraphicsPath();
        path.AddArc(bounds.X, bounds.Y, d, d, 180, 90);
        path.AddArc(bounds.Right - d, bounds.Y, d, d, 270, 90);
        path.AddArc(bounds.Right - d, bounds.Bottom - d, d, d, 0, 90);
        path.AddArc(bounds.X, bounds.Bottom - d, d, d, 90, 90);
        path.CloseFigure();
        return path;
    }

    static Rectangle FindContentBounds(Bitmap bmp)
    {
        int w = bmp.Width, h = bmp.Height;
        int minX = w, minY = h, maxX = -1, maxY = -1;
        for (int y = 0; y < h; y++)
        {
            for (int x = 0; x < w; x++)
            {
                if (bmp.GetPixel(x, y).A > 16)
                {
                    if (x < minX) minX = x;
                    if (y < minY) minY = y;
                    if (x > maxX) maxX = x;
                    if (y > maxY) maxY = y;
                }
            }
        }
        if (maxX < minX || maxY < minY) return new Rectangle(0, 0, w, h);
        return new Rectangle(minX, minY, maxX - minX + 1, maxY - minY + 1);
    }

    /// Saves the icon of `file` as a clean, uniform `size` x `size` rounded-square PNG.
    public static string Save(string file, int index, string png, int size)
    {
        Bitmap src = null;
        try
        {
            string ext = Path.GetExtension(file).ToLowerInvariant();
            if (ext == ".png" || ext == ".jpg" || ext == ".jpeg" || ext == ".bmp")
            {
                using (var loaded = Image.FromFile(file))
                {
                    src = new Bitmap(loaded);
                }
            }
            else
            {
                src = Extract(file, index);
            }
            if (src == null) return "no icon found";

            Rectangle crop = FindContentBounds(src);
            using (var scaled = new Bitmap(size, size, PixelFormat.Format32bppArgb))
            using (var dst = new Bitmap(size, size, PixelFormat.Format32bppArgb))
            {
                using (var g = Graphics.FromImage(scaled))
                {
                    g.InterpolationMode = InterpolationMode.HighQualityBicubic;
                    g.PixelOffsetMode = PixelOffsetMode.HighQuality;
                    g.SmoothingMode = SmoothingMode.AntiAlias;
                    g.CompositingQuality = CompositingQuality.HighQuality;
                    g.Clear(Color.Transparent);
                    g.DrawImage(src, new Rectangle(0, 0, size, size), crop, GraphicsUnit.Pixel);
                }

                // Clip into a smooth anti-aliased rounded square so every icon has uniform shape & size.
                float radius = size * 0.22f;
                using (var g2 = Graphics.FromImage(dst))
                using (var path = RoundedRect(new RectangleF(0.5f, 0.5f, size - 1f, size - 1f), radius))
                using (var brush = new TextureBrush(scaled))
                {
                    g2.SmoothingMode = SmoothingMode.AntiAlias;
                    g2.PixelOffsetMode = PixelOffsetMode.HighQuality;
                    g2.CompositingQuality = CompositingQuality.HighQuality;
                    g2.Clear(Color.Transparent);
                    g2.FillPath(brush, path);
                }

                string tmp = png + ".tmp";
                dst.Save(tmp, ImageFormat.Png);
                if (File.Exists(png)) File.Delete(png);
                File.Move(tmp, png);
            }
            return "ok";
        }
        catch (Exception e) { return e.Message; }
        finally { if (src != null) src.Dispose(); }
    }

    static Bitmap Extract(string file, int index)
    {
        var handles = new IntPtr[1];
        var ids = new uint[1];
        uint n = PrivateExtractIcons(file, index, 256, 256, handles, ids, 1, 0);
        if (n != 0 && n != 0xFFFFFFFF && handles[0] != IntPtr.Zero)
        {
            try { return FromHicon(handles[0]); } finally { DestroyIcon(handles[0]); }
        }
        var info = new SHFILEINFO();
        IntPtr r = SHGetFileInfo(file, 0, ref info, (uint)Marshal.SizeOf(typeof(SHFILEINFO)), SHGFI_ICON | SHGFI_LARGEICON);
        if (r != IntPtr.Zero && info.hIcon != IntPtr.Zero)
        {
            try { return FromHicon(info.hIcon); } finally { DestroyIcon(info.hIcon); }
        }
        return null;
    }

    static Bitmap FromHicon(IntPtr hIcon)
    {
        ICONINFO ii;
        if (!GetIconInfo(hIcon, out ii)) return null;
        try
        {
            if (ii.hbmColor == IntPtr.Zero)
            {
                using (var ic = Icon.FromHandle(hIcon)) return ic.ToBitmap();
            }
            var bm = new BITMAP();
            GetObject(ii.hbmColor, Marshal.SizeOf(typeof(BITMAP)), ref bm);
            int w = bm.bmWidth, h = Math.Abs(bm.bmHeight);
            if (w <= 0 || h <= 0) return null;

            byte[] px = Dib(ii.hbmColor, w, h);
            bool hasAlpha = false;
            for (int i = 3; i < px.Length; i += 4) { if (px[i] != 0) { hasAlpha = true; break; } }
            if (!hasAlpha && ii.hbmMask != IntPtr.Zero)
            {
                byte[] mask = Dib(ii.hbmMask, w, h);
                for (int i = 0; i < px.Length; i += 4) px[i + 3] = (byte)(mask[i] == 0 ? 255 : 0);
            }

            var bmp = new Bitmap(w, h, PixelFormat.Format32bppArgb);
            var data = bmp.LockBits(new Rectangle(0, 0, w, h), ImageLockMode.WriteOnly, PixelFormat.Format32bppArgb);
            try
            {
                for (int y = 0; y < h; y++)
                    Marshal.Copy(px, y * w * 4, IntPtr.Add(data.Scan0, y * data.Stride), w * 4);
            }
            finally { bmp.UnlockBits(data); }
            return bmp;
        }
        finally
        {
            if (ii.hbmColor != IntPtr.Zero) DeleteObject(ii.hbmColor);
            if (ii.hbmMask != IntPtr.Zero) DeleteObject(ii.hbmMask);
        }
    }

    static byte[] Dib(IntPtr hbm, int w, int h)
    {
        var bi = new BITMAPINFOHEADER();
        bi.biSize = (uint)Marshal.SizeOf(typeof(BITMAPINFOHEADER));
        bi.biWidth = w;
        bi.biHeight = -h;
        bi.biPlanes = 1;
        bi.biBitCount = 32;
        var buf = new byte[w * h * 4];
        IntPtr dc = GetDC(IntPtr.Zero);
        try { GetDIBits(dc, hbm, 0, (uint)h, buf, ref bi, 0); }
        finally { ReleaseDC(IntPtr.Zero, dc); }
        return buf;
    }

    public static string PickIconFile(string title)
    {
        string picked = null;
        var t = new Thread(() =>
        {
            using (var dlg = new OpenFileDialog())
            {
                dlg.Title = title;
                dlg.Filter = "Images & Icons (*.png;*.jpg;*.jpeg;*.bmp;*.ico;*.exe)|*.png;*.jpg;*.jpeg;*.bmp;*.ico;*.exe|All files (*.*)|*.*";
                if (dlg.ShowDialog() == DialogResult.OK)
                {
                    picked = dlg.FileName;
                }
            }
        });
        t.SetApartmentState(ApartmentState.STA);
        t.Start();
        t.Join();
        return picked;
    }
}

public static class NookBridge
{
    [DllImport("user32.dll")]
    static extern bool SetProcessDPIAware();

    [DllImport("user32.dll")]
    static extern bool GetCursorPos(out POINT pt);

    [DllImport("user32.dll")]
    static extern IntPtr MonitorFromPoint(POINT pt, uint flags);

    [DllImport("user32.dll")]
    static extern bool GetMonitorInfo(IntPtr hMon, ref MONITORINFO mi);

    [DllImport("shcore.dll")]
    static extern int GetDpiForMonitor(IntPtr hMon, int type, out uint dpiX, out uint dpiY);

    [DllImport("user32.dll")]
    static extern short GetAsyncKeyState(int vKey);

    [StructLayout(LayoutKind.Sequential)]
    struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [StructLayout(LayoutKind.Sequential)]
    struct POINT { public int X; public int Y; }

    [StructLayout(LayoutKind.Sequential)]
    struct MONITORINFO
    {
        public int cbSize;
        public RECT rcMonitor;
        public RECT rcWork;
        public uint dwFlags;
    }

    const int VK_LBUTTON = 0x01;
    const int VK_RBUTTON = 0x02;

    static BlockingCollection<string> Queue = new BlockingCollection<string>();
    static bool Started = false;

    public static volatile bool PageOpen = false;
    public static volatile int GameCount = 0;
    public static volatile int Columns = 7;
    public static volatile float BodyWidth = 564f;
    public static volatile float BodyHeight = 268f;
    public static volatile float GridTopOffset = 0f;
    public static volatile float PlacementFrac = 0.5f;

    public static void Start(TextReader stdin)
    {
        if (Started) return;
        Started = true;
        try { SetProcessDPIAware(); } catch { }

        var reader = new Thread(() =>
        {
            try
            {
                while (true)
                {
                    string line = stdin.ReadLine();
                    if (line == null)
                    {
                        Queue.Add("{\"type\":\"shutdown\"}");
                        break;
                    }
                    if (!string.IsNullOrWhiteSpace(line))
                    {
                        Queue.Add(line);
                    }
                }
            }
            catch
            {
                Queue.Add("{\"type\":\"shutdown\"}");
            }
        });
        reader.IsBackground = true;
        reader.Start();

        var watcher = new Thread(MouseLoop);
        watcher.IsBackground = true;
        watcher.Start();
    }

    public static string ReadNext()
    {
        return Queue.Take();
    }

    static void MouseLoop()
    {
        bool wasLeft = false;
        bool wasRight = false;
        while (true)
        {
            if (!PageOpen || GameCount <= 0)
            {
                wasLeft = (GetAsyncKeyState(VK_LBUTTON) & 0x8000) != 0;
                wasRight = (GetAsyncKeyState(VK_RBUTTON) & 0x8000) != 0;
                Thread.Sleep(40);
                continue;
            }

            bool isLeft = (GetAsyncKeyState(VK_LBUTTON) & 0x8000) != 0;
            bool isRight = (GetAsyncKeyState(VK_RBUTTON) & 0x8000) != 0;

            if ((isLeft && !wasLeft) || (isRight && !wasRight))
            {
                int idx = HitTestGrid();
                if (idx >= 0 && idx < GameCount)
                {
                    string btn = (isRight && !wasRight) ? "right" : "left";
                    Queue.Add("{\"type\":\"_mouse\",\"button\":\"" + btn + "\",\"index\":" + idx + "}");
                }
            }

            wasLeft = isLeft;
            wasRight = isRight;
            Thread.Sleep(15);
        }
    }

    static int HitTestGrid()
    {
        POINT pt;
        if (!GetCursorPos(out pt)) return -1;

        IntPtr hMon = MonitorFromPoint(pt, 2);
        var mi = new MONITORINFO();
        mi.cbSize = Marshal.SizeOf(typeof(MONITORINFO));
        if (!GetMonitorInfo(hMon, ref mi)) return -1;

        uint dpiX = 96, dpiY = 96;
        try { GetDpiForMonitor(hMon, 0, out dpiX, out dpiY); } catch { dpiX = 96; }
        if (dpiX == 0) dpiX = 96;
        float scale = dpiX / 96f;

        float px = (pt.X - mi.rcWork.Left) / scale;
        float py = (pt.Y - mi.rcWork.Top) / scale;
        float winW = (mi.rcWork.Right - mi.rcWork.Left) / scale;

        float bodyW = BodyWidth;
        float panelW = bodyW + 32f;
        float lo = 6f;
        float hi = Math.Max(lo, winW - 6f - panelW);
        float panelLeft = lo + (hi - lo) * PlacementFrac;
        float bodyLeft = panelLeft + 16f;
        float bodyTop = 58f; // 6 (top_margin) + 16 (PANEL_PADDING) + 28 (HEADER) + 8 (gap)

        float gx = px - bodyLeft;
        float gy = py - (bodyTop + GridTopOffset);
        if (gx < 0f || gx > bodyW || gy < 0f || gy > (BodyHeight - GridTopOffset)) return -1;

        int cols = Math.Max(1, Columns);
        float gap = 10f;
        float cellW = (bodyW - gap * (cols - 1)) / cols;
        float cellH = cellW; // square 72x72 cards (padding 10*2 + 52px icon)

        int col = (int)Math.Floor(gx / (cellW + gap));
        float inCol = gx - col * (cellW + gap);
        if (col < 0 || col >= cols || inCol > cellW) return -1;

        int row = (int)Math.Floor(gy / (cellH + gap));
        float inRow = gy - row * (cellH + gap);
        if (row < 0 || inRow > cellH) return -1;

        int idx = row * cols + col;
        return (idx >= 0 && idx < GameCount) ? idx : -1;
    }
}
