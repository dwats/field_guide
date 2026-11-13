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
  $el = Find $root $name $CT::MenuItem 3
  if (-not $el) { $el = Find $root $name $CT::Button 3 }
  return $el
}
function Line($win) { return Find $win "Add relation" $CT::ComboBox }

$n = Get-Date -Format HHmmss
# 1. A fresh guide, so the vocabulary starts empty.
Start-Sleep -Seconds 1
Click (Find $win "New guide" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Boards $n"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1

# 2. Stone Block DROPS Cobblestone Block, mined with Pickaxe; the body mentions the Pickaxe.
Click (Find $win "New entry" $CT::Button)
TypeText (Find $win "Title" $CT::Edit) "Stone Block{ENTER}"
Start-Sleep -Seconds 2
TypeText (Line $win) "drops"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
TypeText (FindPrefix $win "Inverse label" $CT::Edit) "dropped by"
Click (Find $win "Add verb" $CT::Button)
TypeText (Line $win) "Cobblestone Block"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
Click (Find $win "Create entry" $CT::Button)
TypeText (Line $win) "mined with"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
TypeText (FindPrefix $win "Inverse label" $CT::Edit) "mines"
Click (Find $win "Add label" $CT::Button)
TypeText (Line $win) "Pickaxe"
Start-Sleep -Milliseconds 800
Click (FindPrefix $win 'Create "Pickaxe"' $CT::Button)
Start-Sleep -Milliseconds 500
$saveBtn = Find $win "Save" $CT::Button 3
if ($saveBtn) { Click $saveBtn } else { TypeText (Line $win) "{ENTER}" }
Start-Sleep -Seconds 2
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
AssertForeground; [System.Windows.Forms.SendKeys]::SendWait("Grey stone, mined with a {[}{[}Pickaxe{]}{]}.")
Start-Sleep -Seconds 2
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b0-entry.png"

# 3. The board list, then a new board.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b1-boards-list.png"
Click (Find $win "New board" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Overview"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\b2-empty-board.png"

# 4. Add one entry from the picker, then everything that matches an empty search.
$search = Find $win "Search entries to add" $CT::Edit
TypeText $search "Stone"
Start-Sleep -Seconds 1
Click (Find $win "Add Stone Block" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b3-one-card.png"
SetValue $search ""
Start-Sleep -Seconds 1
Click (FindPrefix $win "Add all" $CT::Button)
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\b4-all-cards.png"

# 5. The mention layer off and on again.
Click (Find $win "Mentions" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b5-no-mentions.png"
Click (Find $win "Mentions" $CT::Button)

# 6. Keyboard on a card: nudge, remove, undo.
$card = Find $win "Stone Block" $CT::Button
if ($card) { $card.SetFocus(); Start-Sleep -Milliseconds 500 }
Keys "{RIGHT}{RIGHT}{RIGHT}{RIGHT}{DOWN}{DOWN}"
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b6-nudged.png"
Keys "{DELETE}"
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b7-removed.png"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b8-undone.png"

# 7. Fit, then rename from More.
Click (Find $win "Fit the board to the window" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b9-fit.png"
Click (Find $win "More" $CT::Button)
Start-Sleep -Milliseconds 600
Click (MenuItem $win "Rename board...")
TypeText (Find $win "Name" $CT::Edit) "Overview renamed"
Click (Find $win "Rename" $CT::Button)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b10-renamed.png"

# 8. Leave and come back: membership and positions are restored.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot $hwnd "$OutDir\b11-list-again.png"
Click (Find $win "Overview renamed" $CT::Hyperlink)
Start-Sleep -Seconds 2
Shot $hwnd "$OutDir\b12-reopened.png"

if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
Write-Host "done"
