use crate::models::LocalPrinter;
use serde::Deserialize;
use std::{fs, path::Path, process::Command, time::Duration};
use wait_timeout::ChildExt;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PowerShellPrinter {
    name: String,
    #[serde(default)]
    default: bool,
    #[serde(default)]
    network: bool,
    #[serde(default)]
    shared: bool,
    #[serde(default)]
    work_offline: bool,
}

fn quote_ps(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn hidden_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(program);
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn hidden_command(program: &str) -> Command {
    Command::new(program)
}

pub fn list() -> Result<Vec<LocalPrinter>, String> {
    #[cfg(not(windows))]
    {
        return Ok(Vec::new());
    }

    #[cfg(windows)]
    {
        let script = r#"@(Get-CimInstance Win32_Printer | Select-Object Name,Default,Network,Shared,WorkOffline) | ConvertTo-Json -Compress"#;
        let output = hidden_command("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                script,
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            tracing::warn!("Windows no devolvió la lista de impresoras");
            return Err("No fue posible consultar las impresoras".into());
        }
        let raw = String::from_utf8_lossy(&output.stdout);
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        // Single object vs array
        let trimmed = raw.trim();
        let printers: Vec<PowerShellPrinter> = if trimmed.starts_with('[') {
            serde_json::from_str(trimmed).map_err(|e| e.to_string())?
        } else {
            let one: PowerShellPrinter = serde_json::from_str(trimmed).map_err(|e| e.to_string())?;
            vec![one]
        };
        Ok(printers
            .into_iter()
            .map(|p| LocalPrinter {
                name: p.name,
                is_default: p.default,
                is_network: p.network,
                is_shared: p.shared,
                available: !p.work_offline,
            })
            .collect())
    }
}

pub fn print_test(printer_name: &str, locale: &str) -> Result<(), String> {
    // Bitmap ticket (not Out-Printer text) so 50/80 mm thermal keeps full-width lines.
    let (line1, line2, line3, line4) = match locale {
        "it" => (
            "RESTAAPP PRINTER",
            "By RestaAPP",
            "Stampa di prova OK",
            "Servizio pronto",
        ),
        "en" => (
            "RESTAAPP PRINTER",
            "By RestaAPP",
            "Test print OK",
            "Service ready",
        ),
        "fr" => (
            "RESTAAPP PRINTER",
            "By RestaAPP",
            "Test impression OK",
            "Service pret",
        ),
        _ => (
            "RESTAAPP PRINTER",
            "By RestaAPP",
            "Prueba realizada OK",
            "Servicio listo",
        ),
    };
    let path = std::env::temp_dir().join("restaapp-printer-test.png");
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$w = 576; $h = 320
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::White)
$g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::SingleBitPerPixelGridFit
$font = New-Object System.Drawing.Font 'Consolas', 18, ([System.Drawing.FontStyle]::Bold)
$font2 = New-Object System.Drawing.Font 'Consolas', 14
$brush = [System.Drawing.Brushes]::Black
$y = 24
foreach ($t in @('{l1}','{l2}','------------------------','{l3}','{l4}','50/80 mm thermal')) {{
  $g.DrawString($t, $(if ($y -lt 60) {{ $font }} else {{ $font2 }}), $brush, 16, $y)
  $y += 36
}}
$bmp.Save('{path}', [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
"#,
        l1 = line1.replace('\'', "''"),
        l2 = line2.replace('\'', "''"),
        l3 = line3.replace('\'', "''"),
        l4 = line4.replace('\'', "''"),
        path = quote_ps(&path.to_string_lossy()),
    );
    run_ps(&script)?;
    let result = print_file_with_options(&path, printer_name, 1, Some("thermal80mm"), false);
    let _ = fs::remove_file(&path);
    result
}

/// Print PNG/JPG/PDF using System.Drawing.Printing (GDI path) — more reliable than mspaint /pt.
pub fn print_file(path: &Path, printer_name: &str) -> Result<(), String> {
    print_file_with_options(path, printer_name, 1, None, false)
}

pub fn print_file_with_options(
    path: &Path,
    printer_name: &str,
    copies: u32,
    print_format: Option<&str>,
    open_drawer: bool,
) -> Result<(), String> {
    if !path.exists() {
        return Err("El documento no está disponible".into());
    }
    let extension = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();
    let copies = copies.clamp(1, 5);
    // RestApp formats + aliases (clientes usan 50 mm y 80 mm de forma habitual)
    let width_mm = match print_format.unwrap_or("thermal80mm") {
        "thermal50mm" | "thermal56mm" | "50mm" | "58mm" => 50,
        "thermal112mm" | "112mm" => 112,
        "thermal80mm" | "80mm" => 80,
        other if other.contains("50") || other.contains("56") || other.contains("58") => 50,
        other if other.contains("112") => 112,
        _ => 80,
    };

    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "bmp" | "gif") {
        // Primary: RAW ESC/POS 203dpi 1:1 (sharp, no GDI driver blur). Fallback GDI.
        if let Err(raw_err) = print_image_escpos_raw(path, printer_name, copies, width_mm) {
            tracing::warn!("RAW ESC/POS falló ({}), intento GDI", raw_err);
            print_image_gdi(path, printer_name, copies, width_mm)?;
        }
    } else if extension == "pdf" {
        print_pdf(path, printer_name, copies)?;
    } else {
        // text / raw
        let script = format!(
            "Get-Content -LiteralPath '{}' -Raw | Out-Printer -Name '{}'",
            quote_ps(&path.to_string_lossy()),
            quote_ps(printer_name)
        );
        run_ps(&script)?;
    }

    if open_drawer {
        let _ = open_cash_drawer(printer_name);
    }
    Ok(())
}

/// RAW ESC/POS raster at native 203 DPI — bypasses GDI driver resampling (main blur source).
fn print_image_escpos_raw(
    path: &Path,
    printer_name: &str,
    copies: u32,
    width_mm: u32,
) -> Result<(), String> {
    // 80mm printable ≈ 576 dots; 50mm ≈ 384 dots @ 203 DPI
    let target_dots: u32 = match width_mm {
        0..=55 => 384,  // 50/58 mm class
        56..=90 => 576, // 80 mm class (POS-80C paper width 72.1mm ≈ 576 dots)
        _ => 832,       // 112 mm-ish
    };
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$path = '{path}'
$printer = '{printer}'
$copies = {copies}
$targetW = {target_dots}

function Get-RawHelper {{
  if (-not ('RawPrinterHelper2' -as [type])) {{
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class RawPrinterHelper2 {{
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Ansi)]
  public class DOCINFOA {{
    [MarshalAs(UnmanagedType.LPStr)] public string pDocName;
    [MarshalAs(UnmanagedType.LPStr)] public string pOutputFile;
    [MarshalAs(UnmanagedType.LPStr)] public string pDataType;
  }}
  [DllImport("winspool.Drv", EntryPoint="OpenPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool OpenPrinter([MarshalAs(UnmanagedType.LPStr)] string szPrinter, out IntPtr hPrinter, IntPtr pd);
  [DllImport("winspool.Drv", EntryPoint="ClosePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool ClosePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartDocPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartDocPrinter(IntPtr hPrinter, Int32 level, [In, MarshalAs(UnmanagedType.LPStruct)] DOCINFOA di);
  [DllImport("winspool.Drv", EntryPoint="EndDocPrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndDocPrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="EndPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="WritePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool WritePrinter(IntPtr hPrinter, IntPtr pBytes, Int32 dwCount, out Int32 dwWritten);
  public static bool SendBytes(string printerName, byte[] bytes) {{
    IntPtr hPrinter;
    if (!OpenPrinter(printerName, out hPrinter, IntPtr.Zero)) return false;
    var di = new DOCINFOA();
    di.pDocName = "RestaAPP RAW";
    di.pDataType = "RAW";
    if (!StartDocPrinter(hPrinter, 1, di)) {{ ClosePrinter(hPrinter); return false; }}
    if (!StartPagePrinter(hPrinter)) {{ EndDocPrinter(hPrinter); ClosePrinter(hPrinter); return false; }}
    IntPtr p = Marshal.AllocCoTaskMem(bytes.Length);
    Marshal.Copy(bytes, 0, p, bytes.Length);
    int written;
    bool ok = WritePrinter(hPrinter, p, bytes.Length, out written);
    Marshal.FreeCoTaskMem(p);
    EndPagePrinter(hPrinter);
    EndDocPrinter(hPrinter);
    ClosePrinter(hPrinter);
    return ok && written > 0;
  }}
}}
"@
  }}
}}

Get-RawHelper

$src = [System.Drawing.Image]::FromFile($path)
try {{
  $srcW = $src.Width; $srcH = $src.Height
  if ($srcW -lt 1 -or $srcH -lt 1) {{ throw 'EMPTY_IMAGE' }}
  $targetH = [Math]::Max(1, [int][Math]::Round($srcH * ($targetW / [double]$srcW)))
  if ($targetH -gt 2400) {{ $targetH = 2400 }}

  # Exact thermal width, nearest-neighbor only (no soft blur)
  $bmp = New-Object System.Drawing.Bitmap ([int]$targetW), ([int]$targetH), ([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.Clear([System.Drawing.Color]::White)
  $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
  $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::None
  $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
  $g.DrawImage($src, (New-Object System.Drawing.Rectangle 0,0,$targetW,$targetH), 0,0,$srcW,$srcH, [System.Drawing.GraphicsUnit]::Pixel)
  $g.Dispose()

  # Pack monochrome ESC/POS raster (GS v 0) via LockBits (fast + sharp threshold)
  $widthBytes = [int][Math]::Ceiling($targetW / 8.0)
  # ESC@ (2) + GS v 0 header (8) + raster + feed/cut (7)
  $dataSize = 10 + ($widthBytes * $targetH) + 7
  $bytes = New-Object byte[] $dataSize
  $i = 0
  # ESC @
  $bytes[$i++] = 0x1B; $bytes[$i++] = 0x40
  # GS v 0 m=0  (raster bit image)
  $bytes[$i++] = 0x1D; $bytes[$i++] = 0x76; $bytes[$i++] = 0x30; $bytes[$i++] = 0x00
  $bytes[$i++] = [byte]($widthBytes -band 0xFF)
  $bytes[$i++] = [byte](($widthBytes -shr 8) -band 0xFF)
  $bytes[$i++] = [byte]($targetH -band 0xFF)
  $bytes[$i++] = [byte](($targetH -shr 8) -band 0xFF)

  $rect = New-Object System.Drawing.Rectangle 0, 0, $targetW, $targetH
  $bmpData = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
  try {{
    $stride = $bmpData.Stride
    $scan0 = $bmpData.Scan0
    $row = New-Object byte[] $stride
    for ($y = 0; $y -lt $targetH; $y++) {{
      $rowPtr = [IntPtr]::new($scan0.ToInt64() + [int64]$y * [int64]$stride)
      [System.Runtime.InteropServices.Marshal]::Copy($rowPtr, $row, 0, $stride)
      for ($xb = 0; $xb -lt $widthBytes; $xb++) {{
        $b = 0
        for ($bit = 0; $bit -lt 8; $bit++) {{
          $x = $xb * 8 + $bit
          if ($x -lt $targetW) {{
            $p = $x * 3
            # BGR order in 24bpp
            $lum = (0.114 * $row[$p]) + (0.587 * $row[$p+1]) + (0.299 * $row[$p+2])
            if ($lum -lt 160) {{ $b = $b -bor (0x80 -shr $bit) }}
          }}
        }}
        $bytes[$i++] = [byte]$b
      }}
    }}
  }} finally {{
    $bmp.UnlockBits($bmpData)
    $bmp.Dispose()
  }}

  # Feed + partial cut
  $bytes[$i++] = 0x1B; $bytes[$i++] = 0x64; $bytes[$i++] = 0x04
  $bytes[$i++] = 0x1D; $bytes[$i++] = 0x56; $bytes[$i++] = 0x41; $bytes[$i++] = 0x03
  if ($i -lt $bytes.Length) {{
    $trim = New-Object byte[] $i
    [Array]::Copy($bytes, $trim, $i)
    $bytes = $trim
  }}

  for ($c = 1; $c -le $copies; $c++) {{
    $ok = [RawPrinterHelper2]::SendBytes($printer, $bytes)
    if (-not $ok) {{ throw 'RAW_SEND_FAILED' }}
  }}
}} finally {{
  $src.Dispose()
}}
"#,
        path = quote_ps(&path.to_string_lossy()),
        printer = quote_ps(printer_name),
        copies = copies,
        target_dots = target_dots,
    );
    match run_ps(&script) {
        Ok(()) => {
            tracing::info!(
                "Impresión RAW ESC/POS OK ({}mm / {} dots) → {}",
                width_mm,
                target_dots,
                printer_name
            );
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn print_image_gdi(path: &Path, printer_name: &str, copies: u32, width_mm: u32) -> Result<(), String> {
    // Thermal 50/80 mm — must FIT inside printable area (never overflow right edge).
    // Use Millimeter PageUnit + scale to min(configured mm, HardMargin printable width).
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$path = '{path}'
$printer = '{printer}'
$copies = {copies}
$configuredMm = [double]{width_mm}
$img = [System.Drawing.Image]::FromFile($path)
try {{
  $aspect = if ($img.Width -gt 0) {{ [double]$img.Height / [double]$img.Width }} else {{ 1.5 }}
  # Paper size ~ configured roll (+ small slack height)
  $estH = [Math]::Max(40.0, ($configuredMm - 2.0) * $aspect + 8.0)
  $widthHi = [Math]::Max(80, [int][Math]::Round(($configuredMm / 25.4) * 100.0))
  $heightHi = [Math]::Max(150, [int][Math]::Round(($estH / 25.4) * 100.0) + 40)
  if ($heightHi -gt 20000) {{ $heightHi = 20000 }}

  for ($c = 1; $c -le $copies; $c++) {{
    $pd = New-Object System.Drawing.Printing.PrintDocument
    $pd.PrinterSettings.PrinterName = $printer
    if (-not $pd.PrinterSettings.IsValid) {{ throw 'INVALID_PRINTER' }}
    $pd.DocumentName = 'RestaAPP Print'
    $pd.OriginAtMargins = $false
    $pd.DefaultPageSettings.Margins = New-Object System.Drawing.Printing.Margins(0,0,0,0)
    $pd.DefaultPageSettings.Landscape = $false
    $custom = New-Object System.Drawing.Printing.PaperSize(('RestaAPP ' + [int]$configuredMm + 'mm'), $widthHi, $heightHi)
    $pd.DefaultPageSettings.PaperSize = $custom

    $imgRef = $img
    $cfgMm = $configuredMm
    $pd.add_PrintPage({{
      param($sender, $e)
      $g = $e.Graphics
      $g.PageUnit = [System.Drawing.GraphicsUnit]::Millimeter
      $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
      $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
      $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::None
      $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighSpeed
      $g.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy

      # Hard margins from driver (true printable box) in mm
      $hardL = 0.0; $hardT = 0.0; $hardR = 0.0; $hardB = 0.0
      try {{
        $hardL = $e.PageSettings.HardMarginX / 100.0 * 25.4
        $hardT = $e.PageSettings.HardMarginY / 100.0 * 25.4
      }} catch {{}}
      $pageWmm = $e.PageBounds.Width   # already mm because PageUnit=Millimeter
      $pageHmm = $e.PageBounds.Height
      # Safety: if PageBounds still looks like 1/100" (huge numbers), convert
      if ($pageWmm -gt 500) {{
        $pageWmm = $pageWmm / 100.0 * 25.4
        $pageHmm = $pageHmm / 100.0 * 25.4
      }}

      $availW = [Math]::Max(30.0, $pageWmm - $hardL - 1.5)
      $availH = [Math]::Max(30.0, $pageHmm - $hardT - 1.5)
      # Never wider than configured roll (50 or 80) minus edge safety
      $maxW = [Math]::Min($availW, [Math]::Max(40.0, $cfgMm - 3.0))
      $maxH = $availH

      # Fit image entirely inside box (contain), keep aspect — no crop, no overflow
      $imgAspect = if ($imgRef.Width -gt 0) {{ [double]$imgRef.Height / [double]$imgRef.Width }} else {{ 1.5 }}
      $drawW = $maxW
      $drawH = $drawW * $imgAspect
      if ($drawH -gt $maxH -and $maxH -gt 10) {{
        $drawH = $maxH
        $drawW = $drawH / $imgAspect
      }}

      $x = $hardL + 0.5
      $y = $hardT + 0.5
      $dest = New-Object System.Drawing.RectangleF ([float]$x), ([float]$y), ([float]$drawW), ([float]$drawH)
      $src = New-Object System.Drawing.Rectangle 0, 0, $imgRef.Width, $imgRef.Height
      $g.DrawImage($imgRef, $dest, $src, [System.Drawing.GraphicsUnit]::Pixel)
      $e.HasMorePages = $false
    }})
    $pd.Print()
    $pd.Dispose()
  }}
}} finally {{
  $img.Dispose()
}}
"#,
        path = quote_ps(&path.to_string_lossy()),
        printer = quote_ps(printer_name),
        copies = copies,
        width_mm = width_mm,
    );
    run_ps(&script)
}

fn print_pdf(path: &Path, printer_name: &str, copies: u32) -> Result<(), String> {
    // Prefer Adobe-less PrintTo; fall back to Shell
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$path = '{path}'
$printer = '{printer}'
$copies = {copies}
for ($c = 1; $c -le $copies; $c++) {{
  try {{
    $p = Start-Process -FilePath $path -Verb PrintTo -ArgumentList ('"'+$printer+'"') -PassThru -WindowStyle Hidden
    if ($p) {{
      $p.WaitForExit(20000) | Out-Null
      if (-not $p.HasExited) {{ $p.Kill() }}
    }}
  }} catch {{
    # Fallback mspaint-less: use default association
    Start-Process -FilePath $path -Verb Print -WindowStyle Hidden | Out-Null
    Start-Sleep -Seconds 8
  }}
}}
"#,
        path = quote_ps(&path.to_string_lossy()),
        printer = quote_ps(printer_name),
        copies = copies,
    );
    run_ps(&script)
}

/// ESC/POS open drawer pulse (pin 2 standard).
pub fn open_cash_drawer(printer_name: &str) -> Result<(), String> {
    // ESC p m t1 t2
    let raw_path = std::env::temp_dir().join("restaapp-drawer.bin");
    let bytes: [u8; 5] = [0x1B, b'p', 0x00, 0x19, 0xFA];
    fs::write(&raw_path, bytes).map_err(|e| e.to_string())?;
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
$bytes = [System.IO.File]::ReadAllBytes('{path}')
$printer = '{printer}'
# Send raw via Win32 WritePrinter when possible
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class RawPrinterHelper {{
  [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Ansi)]
  public class DOCINFOA {{
    [MarshalAs(UnmanagedType.LPStr)] public string pDocName;
    [MarshalAs(UnmanagedType.LPStr)] public string pOutputFile;
    [MarshalAs(UnmanagedType.LPStr)] public string pDataType;
  }}
  [DllImport("winspool.Drv", EntryPoint="OpenPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool OpenPrinter([MarshalAs(UnmanagedType.LPStr)] string szPrinter, out IntPtr hPrinter, IntPtr pd);
  [DllImport("winspool.Drv", EntryPoint="ClosePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool ClosePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartDocPrinterA", SetLastError=true, CharSet=CharSet.Ansi, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartDocPrinter(IntPtr hPrinter, Int32 level, [In, MarshalAs(UnmanagedType.LPStruct)] DOCINFOA di);
  [DllImport("winspool.Drv", EntryPoint="EndDocPrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndDocPrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="StartPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool StartPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="EndPagePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool EndPagePrinter(IntPtr hPrinter);
  [DllImport("winspool.Drv", EntryPoint="WritePrinter", SetLastError=true, ExactSpelling=true, CallingConvention=CallingConvention.StdCall)]
  public static extern bool WritePrinter(IntPtr hPrinter, IntPtr pBytes, Int32 dwCount, out Int32 dwWritten);
  public static bool SendBytes(string printerName, byte[] bytes) {{
    IntPtr hPrinter;
    if (!OpenPrinter(printerName.Normalize(), out hPrinter, IntPtr.Zero)) return false;
    var di = new DOCINFOA();
    di.pDocName = "RestaAPP Drawer";
    di.pDataType = "RAW";
    if (!StartDocPrinter(hPrinter, 1, di)) {{ ClosePrinter(hPrinter); return false; }}
    StartPagePrinter(hPrinter);
    IntPtr p = Marshal.AllocCoTaskMem(bytes.Length);
    Marshal.Copy(bytes, 0, p, bytes.Length);
    int written;
    bool ok = WritePrinter(hPrinter, p, bytes.Length, out written);
    Marshal.FreeCoTaskMem(p);
    EndPagePrinter(hPrinter);
    EndDocPrinter(hPrinter);
    ClosePrinter(hPrinter);
    return ok;
  }}
}}
"@
[RawPrinterHelper]::SendBytes($printer, $bytes) | Out-Null
"#,
        path = quote_ps(&raw_path.to_string_lossy()),
        printer = quote_ps(printer_name),
    );
    let result = run_ps(&script);
    let _ = fs::remove_file(raw_path);
    result
}

fn run_ps(script: &str) -> Result<(), String> {
    let output = hidden_command("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Impresión PowerShell falló: {}", err);
        // Fallback mspaint for images if GDI path failed
        Err("La impresora no completó el documento".into())
    }
}

/// Fallback: mspaint /pt if GDI path fails (legacy).
pub fn print_file_fallback_paint(path: &Path, printer_name: &str) -> Result<(), String> {
    let mut child = hidden_command("mspaint.exe")
        .arg("/pt")
        .arg(path)
        .arg(printer_name)
        .spawn()
        .map_err(|e| e.to_string())?;
    match child
        .wait_timeout(Duration::from_secs(18))
        .map_err(|e| e.to_string())?
    {
        Some(status) if status.success() => Ok(()),
        Some(_) => Err("La impresora no completó el documento".into()),
        None => {
            let _ = child.kill();
            Ok(())
        }
    }
}
