# P0-03 smoke (v2): real keys on the real host, whose default input method
# is Microsoft Pinyin. 'x' opens an IME composition (provisional document
# transaction), ENTER commits it (ImeCommit transaction); both go through
# EditTransaction. Idle evidence via two F12 dumps after the composition is
# closed. Produces p0-03-*.log/png/meta.txt in C:\markit-g0.
$ErrorActionPreference = 'Continue'
$exe = 'C:\markit-g0\markit.exe'
$dir = 'C:\markit-g0'
$env:RUST_LOG = 'info'
Remove-Item "$dir\p0-03-smoke.log", "$dir\p0-03-before.png", "$dir\p0-03-after.png", "$dir\p0-03-meta.txt" -ErrorAction SilentlyContinue

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class P0 {
    [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rt, B; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[P0]::SetProcessDPIAware() | Out-Null

function Shot($proc, $path) {
    $proc.Refresh()
    $h = $proc.MainWindowHandle
    $r = New-Object P0+R
    [P0]::GetWindowRect($h, [ref]$r) | Out-Null
    $w = $r.Rt - $r.L; $ht = $r.B - $r.T
    $bmp = New-Object System.Drawing.Bitmap($w, $ht)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size($w, $ht)))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
    "${w}x${ht} at ($($r.L),$($r.T))"
}

$meta = @()
$p = Start-Process -FilePath $exe -RedirectStandardError "$dir\p0-03-smoke.log" -RedirectStandardOutput "$dir\p0-03-stdout.log" -PassThru
Start-Sleep -Milliseconds 3000   # settle past first draw

# 1) initial frame: Markdown visibly styled (screenshot is the evidence)
$meta += "before_shot=$(Shot $p "$dir\p0-03-before.png")"

# 2) one REAL keystroke: 'x' -> IME composition update (transaction 1)
$ws = New-Object -ComObject WScript.Shell
$activated = $ws.AppActivate($p.Id)
$meta += "sendkeys AppActivate=$activated"
Start-Sleep -Milliseconds 300
$ws.SendKeys('x')
Start-Sleep -Milliseconds 800

# 3) ENTER commits the raw pinyin letter (transaction 2, ImeCommit intent)
$ws.SendKeys('{ENTER}')
Start-Sleep -Milliseconds 800
$meta += "after_shot=$(Shot $p "$dir\p0-03-after.png")"

# 4) idle evidence: two F12 dumps 2s apart; draws must NOT advance between them
Start-Sleep -Milliseconds 2000
$ws.SendKeys('{F12}')
Start-Sleep -Milliseconds 2000
$ws.SendKeys('{F12}')
Start-Sleep -Milliseconds 600

# 5) idle CPU/RSS sampling
for ($i = 0; $i -lt 8; $i++) {
    Start-Sleep -Milliseconds 500
    $p.Refresh()
    $meta += "idle_sample cpu_s=$([math]::Round($p.TotalProcessorTime.TotalSeconds,3)) ws_mb=$([math]::Round($p.WorkingSet64/1MB,1)) threads=$($p.Threads.Count)"
}

if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
Start-Sleep -Milliseconds 300

# 6) pixel-change proof: the two window captures must differ
$h1 = (Get-FileHash "$dir\p0-03-before.png" -Algorithm SHA256).Hash
$h2 = (Get-FileHash "$dir\p0-03-after.png" -Algorithm SHA256).Hash
$meta += "before_sha256=$h1"
$meta += "after_sha256=$h2"
$meta += "pixels_changed=$($h1 -ne $h2)"
$meta | Out-File -Encoding utf8 "$dir\p0-03-meta.txt"
