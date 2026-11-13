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

function SetValue($el, $text) {
  if (-not $el) { return }
  try { $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($text); Start-Sleep -Milliseconds 500 } catch { Write-Host "SETVALUE FAILED: $($el.Current.Name)" }
}
function MenuItem($root, $name) {
  $el = Find $root $name $CT::MenuItem 2
  if (-not $el) { $el = Find $root $name $CT::RadioButton 2 }
  if (-not $el) { $el = Find $root $name $CT::Button 2 }
  return $el
}
function Line($win) { return Find $win "Add relation" $CT::ComboBox }
function Line($win) { return Find $win "Add relation" $CT::ComboBox }
function Focus($el) { if ($el) { $el.SetFocus(); Start-Sleep -Milliseconds 600 } else { Write-Host "FOCUS: nothing to focus" } }

$n = Get-Date -Format HHmmss
# 1. A guide with two entries and a board holding both.
Start-Sleep -Seconds 1
Click (Find $win "New guide" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Notes $n"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1
Click (Find $win "New entry" $CT::Button)
TypeText (Find $win "Title" $CT::Edit) "Stone Block{ENTER}"
Start-Sleep -Seconds 2
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "New board" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Clues"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 2
Click (FindPrefix $win "Add all" $CT::Button)
Start-Sleep -Seconds 1

# 2. A note with a task list, saved with Ctrl+Enter.
Click (Find $win "New note" $CT::Button)
Start-Sleep -Seconds 1
$editor = Find $win "Note text in Markdown" $CT::Edit 4
Write-Host ("editor focused: " + $AE::FocusedElement.Current.Name)
AssertForeground; [System.Windows.Forms.SendKeys]::SendWait("## Todo{ENTER}- {[} {]} Mine the stone{ENTER}- {[}x{]} Find a pickaxe{ENTER}{ENTER}See {[}{[}Stone Block{]}{]}.")
Start-Sleep -Milliseconds 500
Keys "^{ENTER}"
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\n1-note.png"

# 3. The first check box flips and the change survives a reload of the view.
$c = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::CheckBox)
$boxes = $win.FindAll([System.Windows.Automation.TreeScope]::Descendants, $c)
Write-Host "check boxes: $($boxes.Count)"
$box = $null
foreach ($b in $boxes) { if ($b.Current.Name -ne "Stubs only") { $box = $b; break } }
if ($box) {
  try { $box.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle() } catch { Write-Host "toggle failed: $_" }
} else { Write-Host "no task box found" }
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\n2-checked.png"

# 4. Recolor the note from the panel (the note is selected after saving).
Focus (FindPrefix $win "Note: Todo" $CT::Button)
Click (Find $win "Green" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\n3-green.png"

# 5. Tie a string from the keyboard: select the card, Tie a string, choose the note with Enter.
Focus (Find $win "Stone Block" $CT::Button)
Click (Find $win "Tie a string" $CT::Button)
Start-Sleep -Milliseconds 800
Shot $hwnd "$OutDir\n4-tying.png"
Focus (FindPrefix $win "Note: Todo" $CT::Button)
Keys "{ENTER}"
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\n5-string.png"
TypeText (Find $win "String label" $CT::Edit) "needs"
Keys "{TAB}"
Start-Sleep -Seconds 1
Click (Find $win "Blue" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\n6-labeled.png"

# 6. Backgrounds from More.
Click (Find $win "More" $CT::Button)
Start-Sleep -Milliseconds 600
Click (MenuItem $win "Background: Cork")
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\n7-cork.png"
Click (Find $win "More" $CT::Button)
Start-Sleep -Milliseconds 600
Click (MenuItem $win "Background: Grid")
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\n8-grid.png"

# 7. Delete the note with the keyboard: its string goes too; Undo brings both back.
Focus (FindPrefix $win "Note: Todo" $CT::Button)
Keys "{DELETE}"
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\n9-deleted.png"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\n10-undone.png"

# 7b. Paste a picture onto the board: it lands in a polaroid frame at the middle; resize it from a corner via keys is not available, so just select it.
$bmp = New-Object System.Drawing.Bitmap 120, 80
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$gfx.Clear([System.Drawing.Color]::Teal)
$gfx.FillRectangle([System.Drawing.Brushes]::Gold, 20, 15, 80, 50)
$gfx.Dispose()
[System.Windows.Forms.Clipboard]::SetImage($bmp)
Focus (Find $win "Board canvas" $CT::Pane 3)
if (-not ($AE::FocusedElement.Current.Name -eq "Board canvas")) { Focus (Find $win "Board canvas" $CT::Custom 3) }
Keys "^v"
Start-Sleep -Seconds 3
Shot $hwnd "$OutDir\img-1.png"
Focus (Find $win "Image" $CT::Button 3)
Click (Find $win "Tie a string" $CT::Button)
Start-Sleep -Milliseconds 600
Focus (Find $win "Stone Block" $CT::Button)
Keys "{ENTER}"
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\img-2.png"

# 8. Leave and come back.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "Clues" $CT::Hyperlink)
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\n11-reopened.png"

if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
Write-Host "done"
