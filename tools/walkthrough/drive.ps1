param([string]$Exe, [string]$OutDir)
$ErrorActionPreference = "Continue"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing, System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@
[Win]::SetProcessDPIAware() | Out-Null
$AE = [System.Windows.Automation.AutomationElement]
$CT = [System.Windows.Automation.ControlType]

function Shot($hwnd, $path) {
  $rect = New-Object Win+RECT
  [Win]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
  $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc(); [Win]::PrintWindow($hwnd, $hdc, 2) | Out-Null; $g.ReleaseHdc($hdc)
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  Write-Host "shot $path"
}

function Find($root, $name, $type, $seconds = 6) {
  $deadline = (Get-Date).AddSeconds($seconds)
  do {
    $c1 = New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, $name)
    $c2 = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type)
    $and = New-Object System.Windows.Automation.AndCondition($c1, $c2)
    $el = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $and)
    if ($el) { return $el }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  Write-Host "NOT FOUND: $name ($($type.ProgrammaticName))"
  $c = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type)
  $names = @()
  foreach ($el in $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)) { $names += $el.Current.Name }
  Write-Host ("  present: " + ($names -join " | ") + "   focused: " + $AE::FocusedElement.Current.Name)
  return $null
}

function Click($el) {
  if (-not $el) { return }
  $done = $false
  foreach ($attempt in 1..3) {
    try {
      if ($attempt -eq 1) { $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }
      elseif ($attempt -eq 2) { $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle() }
      else {
        $ec = $el.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
        if ($ec.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Expanded) { $ec.Collapse() } else { $ec.Expand() }
      }
      $done = $true; break
    } catch { }
  }
  if (-not $done) { Write-Host "CLICK FAILED: $($el.Current.Name)" }
  Start-Sleep -Milliseconds 800
}

function Keys($text) { AssertForeground; [System.Windows.Forms.SendKeys]::SendWait($text); Start-Sleep -Milliseconds 900 }

function AssertForeground() {
  if ([Win]::GetForegroundWindow() -ne $script:hwnd) {
    Write-Host "ABORTED: another window took focus; no keys were sent to it"
    if ($script:proc -and -not $script:proc.HasExited) { Stop-Process -Id $script:proc.Id -Force }
    exit 3
  }
}

function TypeText($el, $text) {
  if (-not $el) { return }
  if (-not $el.Current.HasKeyboardFocus) { $el.SetFocus(); Start-Sleep -Milliseconds 400 }
  AssertForeground; [System.Windows.Forms.SendKeys]::SendWait($text); Start-Sleep -Milliseconds 600
  $plain = $text -replace '\{ENTER\}', ''
  if (-not $plain) { return }
  try {
    $vp = $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $value = $vp.Current.Value
    if (-not $value.EndsWith($plain)) {
      Write-Host "RETYPED: field held [$value], wanted [$plain]"
      $vp.SetValue($plain); Start-Sleep -Milliseconds 400
      if ($text -like '*{ENTER}*') { AssertForeground; [System.Windows.Forms.SendKeys]::SendWait('{ENTER}'); Start-Sleep -Milliseconds 600 }
    }
  } catch { }
}

$script:proc = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru; $proc = $script:proc
Start-Sleep -Seconds 7
$proc.Refresh()
$script:hwnd = $proc.MainWindowHandle; $hwnd = $script:hwnd
[Win]::SetForegroundWindow($hwnd) | Out-Null
$cond = New-Object System.Windows.Automation.PropertyCondition($AE::ProcessIdProperty, $proc.Id)
$win = $AE::RootElement.FindFirst([System.Windows.Automation.TreeScope]::Children, $cond)
if (-not $win) { Write-Host "no automation root"; Stop-Process -Id $proc.Id -Force; exit 1 }

function FindPrefix($root, $prefix, $type) {
  $c = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type)
  foreach ($el in $root.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)) {
    if ($el.Current.Name.StartsWith($prefix)) { return $el }
  }
  return $null
}

# 1. Open the existing guide (created by the previous run) or create one. The poster grid
# arrives a moment after the window; wait for it.
$existing = $null
foreach ($i in 1..20) {
  $existing = FindPrefix $win "Minecraft" $CT::Hyperlink
  if ($existing) { break }
  Start-Sleep -Milliseconds 500
}
if ($existing) {
  Click $existing
} else {
  Click (Find $win "New guide" $CT::Button)
  TypeText (Find $win "Name" $CT::Edit) "Minecraft"
  TypeText (Find $win "Description" $CT::Edit) "Survival notes"
  Click (Find $win "Create" $CT::Button)
}
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\1-guide.png"

$n = Get-Date -Format HHmmss
# 2. Create the link target, go back, create the linking note.
Click (Find $win "New entry" $CT::Button)
TypeText (Find $win "Title" $CT::Edit) "Oak Log $n{ENTER}"
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\2-oak-new.png"
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 1
Click (Find $win "New entry" $CT::Button)
TypeText (Find $win "Title" $CT::Edit) "Stone Block $n{ENTER}"
Start-Sleep -Seconds 2

# 3. Edit mode, type a body with a resolved and an unresolved link.
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
AssertForeground; [System.Windows.Forms.SendKeys]::SendWait("# Description{ENTER}Grey stone. Drops {[}{[}Cobblestone{]}{]} and sits near {[}{[}Oak Log $n{]}{]}.{ENTER}{ENTER}## Notes{ENTER}Hidden: ||a secret||{ENTER}{ENTER}> {[}!NOTE{]}{ENTER}Mined with any pickaxe.")
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\3-edit.png"

# 3b. Paste a bitmap from the clipboard: it is stored as lossless WebP and lands in the body at the caret.
$bmp = New-Object System.Drawing.Bitmap 96, 64
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.Clear([System.Drawing.Color]::Orange)
$gfx.FillEllipse([System.Drawing.Brushes]::Navy, 12, 10, 72, 44)
$gfx.Dispose()
[System.Windows.Forms.Clipboard]::SetImage($bmp)
AssertForeground; [System.Windows.Forms.SendKeys]::SendWait("{ENTER}{ENTER}^v")
Start-Sleep -Seconds 3
Shot $hwnd "$OutDir\3b-paste.png"

# 4. Nest a tag from the note page, then switch to read mode.
Click (Find $win "+ Add tag" $CT::Button)
TypeText (Find $win "Add tag" $CT::ComboBox) "Blocks/Natural{ENTER}"
Start-Sleep -Seconds 1
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\4-read.png"

# 5. Follow the resolved link, then look at the tag editor.
Click (Find $win "Oak Log $n" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\5-oak.png"
Click (Find $win "Minecraft" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "Tag editor" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\6-tags.png"

# 7. Sidebar: pick the Natural tag, the list narrows to the entries under it; then the Trash view.
Click (Find $win "Expand Blocks" $CT::Button)
Click (Find $win "Natural" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\7-sidebar.png"
Click (Find $win "Stone Block $n" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (FindPrefix $win "Trash" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\8-trash.png"

# 7b. The palette: Ctrl+K, type a title, the entry row is highlighted; Enter opens it.
Keys "^k"
TypeText (Find $win "Search and commands" $CT::ComboBox) "Oak Log $n"
Start-Sleep -Milliseconds 800
Shot $hwnd "$OutDir\10-palette.png"
Keys "{ENTER}"
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\11-palette-opened.png"

# 8. Settings: the Appearance section.
Click (Find $win "Settings" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\9-settings.png"

# 6. Second launch should exit and leave the first window.
$second = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru
Start-Sleep -Seconds 4
$second.Refresh()
Write-Host "second launch exited: $($second.HasExited)"
$proc.Refresh()
Write-Host "first still running: $(-not $proc.HasExited)"

if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
Write-Host "done"

