param(
  [string]$Exe,
  [string]$OutDir,
  [ValidateSet("", "dark", "light")][string]$Theme = ""
)
$ErrorActionPreference = "Continue"
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes, System.Drawing, System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int w, int h, bool repaint);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hWnd, IntPtr hdc, uint flags);
}
"@
[Win]::SetProcessDPIAware() | Out-Null
$AE = [System.Windows.Automation.AutomationElement]
$CT = [System.Windows.Automation.ControlType]
$TS = [System.Windows.Automation.TreeScope]

# Every control the script could not find or click counts as a miss. The script keeps going,
# prints each miss, and exits with code 2 when there was one.
$script:misses = 0
function Miss($what) { $script:misses++; Write-Host "MISS: $what" }

# The window's own rendering, title bar included as PrintWindow sees it.
function Shot($name) {
  $rect = New-Object Win+RECT
  [Win]::GetWindowRect($script:hwnd, [ref]$rect) | Out-Null
  $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $hdc = $g.GetHdc(); [Win]::PrintWindow($script:hwnd, $hdc, 2) | Out-Null; $g.ReleaseHdc($hdc)
  $path = Join-Path $OutDir "$name.png"
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  Write-Host "shot $path"
}

# The screen where the window is: shows the title bar as the desktop draws it.
function ScreenShot($name) {
  $rect = New-Object Win+RECT
  [Win]::GetWindowRect($script:hwnd, [ref]$rect) | Out-Null
  $w = $rect.Right - $rect.Left; $h = $rect.Bottom - $rect.Top
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
  $path = Join-Path $OutDir "$name.png"
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  Write-Host "screen $path"
}

# The first element with this exact name, of this control type when one is given.
function Find($root, $name, $type, $seconds = 6, [switch]$Optional) {
  if (-not $root) { return $null }
  $cond = New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, $name)
  if ($type) {
    $c2 = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type)
    $cond = New-Object System.Windows.Automation.AndCondition($cond, $c2)
  }
  $deadline = (Get-Date).AddSeconds($seconds)
  do {
    try { $el = $root.FindFirst($TS::Descendants, $cond) } catch { $el = $null }
    if ($el) { return $el }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  if ($Optional) { return $null }
  $typeName = "any type"
  if ($type) { $typeName = $type.ProgrammaticName }
  Miss "$name ($typeName)"
  if ($type) {
    $c = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type)
    $names = @()
    foreach ($e in $root.FindAll($TS::Descendants, $c)) { $names += $e.Current.Name }
    Write-Host ("  present: " + ($names -join " | ") + "   focused: " + $AE::FocusedElement.Current.Name)
  }
  return $null
}

# The first element whose name starts with the prefix, ignoring case. With -Contains the text
# may sit anywhere in the name (a tree row's name starts with its fold button's).
function FindPrefix($root, $prefix, $type, $seconds = 4, [switch]$Optional, [switch]$Contains) {
  if (-not $root) { return $null }
  $cond = [System.Windows.Automation.Condition]::TrueCondition
  if ($type) { $cond = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $type) }
  $deadline = (Get-Date).AddSeconds($seconds)
  do {
    try {
      foreach ($el in $root.FindAll($TS::Descendants, $cond)) {
        $name = $el.Current.Name
        if ($name.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) { return $el }
        if ($Contains -and $name.IndexOf($prefix, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) { return $el }
      }
    } catch { }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  if (-not $Optional) { Miss "prefix $prefix" }
  return $null
}

# A button, switch, check box, or radio tile whose name starts with the prefix. Settings tiles
# carry their description in the name, so an exact match would miss them.
function FindControl($root, $prefix, $seconds = 4) {
  $kinds = @($CT::Button.Id, $CT::CheckBox.Id, $CT::RadioButton.Id)
  $deadline = (Get-Date).AddSeconds($seconds)
  do {
    try {
      foreach ($el in $root.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
        if ($kinds -contains $el.Current.ControlType.Id -and $el.Current.Name.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) { return $el }
      }
    } catch { }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  Miss "control $prefix"
  return $null
}

# A container with this name (a dialog, a panel, a group), not a button, link, or text of the
# same name. Prefers one that holds buttons.
function Region($name, $seconds = 4, [switch]$Optional) {
  $cond = New-Object System.Windows.Automation.PropertyCondition($AE::NameProperty, $name)
  $skip = @($CT::Button.Id, $CT::Text.Id, $CT::Hyperlink.Id, $CT::TabItem.Id, $CT::Edit.Id, $CT::ComboBox.Id)
  $buttons = New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::Button)
  $deadline = (Get-Date).AddSeconds($seconds)
  do {
    $first = $null
    try {
      foreach ($el in $win.FindAll($TS::Descendants, $cond)) {
        if ($skip -contains $el.Current.ControlType.Id) { continue }
        if (-not $first) { $first = $el }
        if ($el.FindFirst($TS::Descendants, $buttons)) { return $el }
      }
    } catch { }
    if ($first) { return $first }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  if (-not $Optional) { Miss "region $name" }
  return $null
}

# Prints what is under an element. For finding out what a control is called.
function Dump($root, $label) {
  Write-Host "DUMP $label"
  if (-not $root) { return }
  foreach ($el in $root.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
    $kind = $el.Current.ControlType.ProgrammaticName -replace '^ControlType\.', ''
    if ($el.Current.Name) { Write-Host ("  " + $kind + ": " + $el.Current.Name) }
  }
}

function Click($el) {
  if (-not $el) { return }
  $done = $false
  foreach ($attempt in 1..4) {
    try {
      if ($attempt -eq 1) { $el.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke() }
      elseif ($attempt -eq 2) { $el.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle() }
      elseif ($attempt -eq 3) { $el.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern).Select() }
      else {
        $ec = $el.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
        if ($ec.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Expanded) { $ec.Collapse() } else { $ec.Expand() }
      }
      $done = $true; break
    } catch { }
  }
  if (-not $done) { Miss "click on $($el.Current.Name)" }
  Start-Sleep -Milliseconds 800
}

function AssertForeground() {
  if ([Win]::GetForegroundWindow() -ne $script:hwnd) {
    Write-Host "ABORTED: another window took focus; the script stops and closes the app it started"
    if ($script:proc -and -not $script:proc.HasExited) { Stop-Process -Id $script:proc.Id -Force }
    exit 3
  }
}

# Literal text for SendKeys: its special characters are wrapped in braces.
function Esc($text) { return ($text -replace '([+^%~(){}\[\]])', '{$1}') }

# Sends keys in SendKeys notation ({ENTER}, ^k, +{DOWN}). Use Esc for literal text.
function Keys($text) { AssertForeground; [System.Windows.Forms.SendKeys]::SendWait($text); Start-Sleep -Milliseconds 900 }

# Types literal text into a field and checks that the field holds it.
function TypeText($el, $text, [switch]$Enter) {
  if (-not $el) { return }
  if (-not $el.Current.HasKeyboardFocus) { try { $el.SetFocus() } catch { }; Start-Sleep -Milliseconds 400 }
  AssertForeground; [System.Windows.Forms.SendKeys]::SendWait((Esc $text)); Start-Sleep -Milliseconds 600
  try {
    $vp = $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
    $value = $vp.Current.Value
    if (-not $value.EndsWith($text)) {
      Write-Host "RETYPED: field held [$value], wanted [$text]"
      $vp.SetValue($text); Start-Sleep -Milliseconds 400
    }
  } catch { }
  if ($Enter) { Keys "{ENTER}" }
}

function SetValue($el, $text) {
  if (-not $el) { return }
  try { $el.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($text); Start-Sleep -Milliseconds 500 } catch { Miss "set value on $($el.Current.Name)" }
}

# Keyboard focus by UI Automation. Never use it on a board note's editor: a blur saves the note.
function Focus($el) { if ($el) { try { $el.SetFocus() } catch { Miss "focus on $($el.Current.Name)" }; Start-Sleep -Milliseconds 600 } }

function Reveal($el) {
  if (-not $el) { return }
  try { $el.GetCurrentPattern([System.Windows.Automation.ScrollItemPattern]::Pattern).ScrollIntoView() } catch { Write-Host "REVEAL FAILED: $($el.Current.Name)" }
  Start-Sleep -Milliseconds 600
}

# Text through the clipboard. The editor pairs brackets and indents typed keys; a paste is exact.
function PasteText($text) {
  [System.Windows.Forms.Clipboard]::SetText($text)
  Keys "^v"
}

# A picture on the clipboard: an ellipse on a plain background.
function ClipImage($w, $h, $back, $fore) {
  $bmp = New-Object System.Drawing.Bitmap $w, $h
  $gfx = [System.Drawing.Graphics]::FromImage($bmp)
  $gfx.Clear([System.Drawing.Color]::FromName($back))
  $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromName($fore))
  $gfx.FillEllipse($brush, [int]($w * 0.15), [int]($h * 0.15), [int]($w * 0.7), [int]($h * 0.7))
  $brush.Dispose(); $gfx.Dispose()
  [System.Windows.Forms.Clipboard]::SetImage($bmp)
}

# A menu row. Plain rows are MenuItem, radio rows are RadioButton, check rows are CheckBox.
# The exact name wins; otherwise a row whose name contains the text (a check row's name can
# carry its tooltip).
function MenuItem($name) {
  $kinds = @($CT::MenuItem.Id, $CT::RadioButton.Id, $CT::CheckBox.Id, $CT::Button.Id)
  $deadline = (Get-Date).AddSeconds(3)
  do {
    $loose = $null
    try {
      foreach ($el in $win.FindAll($TS::Descendants, [System.Windows.Automation.Condition]::TrueCondition)) {
        if ($kinds -notcontains $el.Current.ControlType.Id) { continue }
        $found = $el.Current.Name
        if ($found -eq $name) { return $el }
        if (-not $loose -and $found.IndexOf($name, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) { $loose = $el }
      }
    } catch { }
    if ($loose) { Write-Host "menu row [$name] matched [$($loose.Current.Name)]"; return $loose }
    Start-Sleep -Milliseconds 300
  } while ((Get-Date) -lt $deadline)
  Miss "menu row $name"
  return $null
}

function IsExpanded($el) {
  try {
    $state = $el.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern).Current.ExpandCollapseState
    return ($state -eq [System.Windows.Automation.ExpandCollapseState]::Expanded)
  } catch { return $false }
}

# Opens the menu behind a button (More, View) unless it is open already.
function OpenMenu($name) {
  $b = Find $win $name $CT::Button
  if ($b -and -not (IsExpanded $b)) { Click $b }
  Start-Sleep -Milliseconds 400
}

function CloseMenu($name) {
  $b = Find $win $name $CT::Button 2 -Optional
  if ($b -and (IsExpanded $b)) { Keys "{ESC}" }
}

# Opens the palette and types. Runs the highlighted row unless -Stay is given.
function Palette($text, [switch]$Stay) {
  Keys "^k"
  $box = Find $win "Search and commands" $CT::ComboBox 2 -Optional
  if (-not $box) {
    Click (FindPrefix $win "Search" $CT::Button)
    $box = Find $win "Search and commands" $CT::ComboBox
  }
  if (-not $box) { return }
  TypeText $box $text
  Start-Sleep -Milliseconds 700
  if (-not $Stay) { Keys "{ENTER}" }
}

function SetTheme($name) {
  if ($name -eq "dark") { Palette "Theme: Dark" }
  elseif ($name -eq "light") { Palette "Theme: Light" }
  Start-Sleep -Milliseconds 800
}

# New entry from anywhere inside a guide. Lands on the new entry in Read.
function NewEntry($title) {
  $button = Find $win "New entry" $CT::Button 2 -Optional
  if ($button) { Click $button } else { Keys "^n" }
  TypeText (Find $win "Title" $CT::Edit) $title -Enter
  Start-Sleep -Seconds 2
}

function OpenSettings() { Click (Find $win "Settings" $CT::Hyperlink); Start-Sleep -Seconds 1 }

# Closes the app the way a user does, so settings are written; kills it only if it stays.
function Finish() {
  if ($script:proc -and -not $script:proc.HasExited) {
    $script:proc.CloseMainWindow() | Out-Null
    if (-not $script:proc.WaitForExit(4000)) { Stop-Process -Id $script:proc.Id -Force }
  }
  Write-Host "done, misses: $($script:misses)"
  if ($script:misses -gt 0) { exit 2 }
  exit 0
}

if (-not $Exe -or -not (Test-Path $Exe)) { Write-Host "no executable at [$Exe]"; exit 1 }
if (-not $OutDir) { Write-Host "no -OutDir given"; exit 1 }
$Exe = (Resolve-Path $Exe).Path
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$OutDir = (Resolve-Path $OutDir).Path
if (Get-Process -Name fieldguide -ErrorAction SilentlyContinue) {
  Write-Host "fieldguide.exe is already running; close it first (the app is single-instance)"
  exit 1
}

$script:proc = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru
$proc = $script:proc
$win = $null
foreach ($i in 1..40) {
  Start-Sleep -Milliseconds 500
  $proc.Refresh()
  if ($proc.HasExited) { break }
  if ($proc.MainWindowHandle -ne [IntPtr]::Zero) {
    $cond = New-Object System.Windows.Automation.PropertyCondition($AE::ProcessIdProperty, $proc.Id)
    $win = $AE::RootElement.FindFirst($TS::Children, $cond)
    if ($win) { break }
  }
}
if (-not $win) {
  Write-Host "no automation root"
  if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
  exit 1
}
$script:hwnd = $proc.MainWindowHandle
$area = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$width = [Math]::Min(1440, $area.Width - 40)
$height = [Math]::Min(960, $area.Height - 40)
[Win]::MoveWindow($script:hwnd, $area.Left + 20, $area.Top + 20, $width, $height, $true) | Out-Null
[Win]::SetForegroundWindow($script:hwnd) | Out-Null

# The first screen is the guides list. It arrives a moment after the window.
if (-not (Find $win "New guide" $CT::Button 15)) { Write-Host "the guides list did not appear"; Finish }
SetTheme $Theme
$n = Get-Date -Format HHmmss

# notes.ps1: board notes in five colors, check boxes, a pasted image in its polaroid frame,
# strings in four colors, the four backgrounds, an index card and a picture card, delete with
# Undo, Dim images, and reopening.

function Panel() { return Region "Board panel" }
function Canvas() { return Find $win "Board canvas" $null }
function Card($title) { return Find (Canvas) $title $CT::Button }
function Note($heading) { return FindPrefix (Canvas) "Note: $heading" $CT::Button }
function Fit() { Click (Find $win "Fit the board to the window" $CT::Button); Start-Sleep -Milliseconds 600 }

# Shift with an arrow moves the focused item 50 units. Steps may be negative; the [int] matters,
# because a bare -5 arrives as text and text compares wrongly with 0.
function Nudge([int]$right, [int]$down) {
  $keys = ""
  if ($right -gt 0) { $keys += "{RIGHT}" * $right } elseif ($right -lt 0) { $keys += "{LEFT}" * (-$right) }
  if ($down -gt 0) { $keys += "{DOWN}" * $down } elseif ($down -lt 0) { $keys += "{UP}" * (-$down) }
  if ($keys) { Keys ("+(" + $keys + ")") }
}

# A new note opens for editing with the caret in it. Escape saves it and returns focus to the
# note, which is then moved and recolored. The editor is never focused by UI Automation.
function NewNote($heading, $lines, $color, [int]$right) {
  Click (Find $win "New note" $CT::Button)
  Start-Sleep -Seconds 1
  if (-not (Find $win "Note text in Markdown" $CT::Edit 4)) { return }
  $keys = Esc "## $heading"
  foreach ($line in $lines) { $keys += "{ENTER}" + (Esc $line) }
  Keys $keys
  Keys "{ESC}"
  Start-Sleep -Seconds 1
  Write-Host ("after Escape, focus is on: " + $AE::FocusedElement.Current.Name)
  if ($AE::FocusedElement.Current.Name -notlike "Note: $heading*") { Focus (Note $heading) }
  Nudge $right 6
  if ($color) { Click (Find (Panel) $color $CT::Button) }
}

# A string from one item to another, tied from the keyboard, then labeled and colored.
function Tie($from, $to, $label, $color) {
  Focus $from
  Click (Find (Panel) "Tie a string" $CT::Button)
  Start-Sleep -Milliseconds 600
  Focus $to
  Keys "{ENTER}"
  Start-Sleep -Seconds 1
  if ($label) { TypeText (Find (Panel) "String label" $CT::Edit) $label; Keys "{TAB}" }
  Click (Find (Panel) $color $CT::Button)
}

function Background($name) {
  OpenMenu "View"
  Click (MenuItem "Background: $name")
  CloseMenu "View"
  Start-Sleep -Milliseconds 600
}

# 1. A guide with a plain entry (an index card) and an entry with a picture (a picture card).
Click (Find $win "New guide" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Notes $n"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1
NewEntry "Stone Block"
NewEntry "Oak Log"
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
ClipImage 240 180 "White" "SaddleBrown"
Keys "^v"
Start-Sleep -Seconds 3
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 1

# 2. A board holding both.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "New board" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Clues"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 2
Click (FindPrefix $win "Add all" $CT::Button)
Start-Sleep -Seconds 1

# 3. The first note, with a task list, a wikilink, and a web link. It stays selected, so the
# screenshot shows its pin and its handles.
NewNote "Yellow" @("- [ ] Mine the stone", "- [x] Find a pickaxe", "", "See [[Stone Block]] and [a site](https://example.com).") $null -15
Fit
Shot "n01-note-selected"

# 4. The first check box flips, and the change is saved.
$boxes = @()
$canvas = Canvas
if ($canvas) { $boxes = @($canvas.FindAll($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::CheckBox)))) }
Write-Host "check boxes in notes: $($boxes.Count)"
if ($boxes.Count -gt 0) {
  try { $boxes[0].GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle() } catch { Miss "toggle on a task box" }
} else { Miss "a task box in the note" }
Start-Sleep -Seconds 2
Shot "n02-task-checked"

# 5. Four more notes, one per remaining color, in a row under the cards.
NewNote "Pink" @("Text and a link to [[Oak Log]].") "Pink" -5
NewNote "Blue" @("Text and a link to [[Oak Log]].") "Blue" 0
NewNote "Green" @("Text and a link to [[Oak Log]].") "Green" 5
NewNote "White" @("Text and a link to [[Oak Log]].") "White" 10
Fit
Shot "n03-five-notes"

# 6. A pasted picture lands in a polaroid frame.
ClipImage 240 160 "White" "Teal"
Focus (Canvas)
Keys "^v"
Start-Sleep -Seconds 3
$image = Find (Canvas) "Image" $CT::Button
Focus $image
Nudge 0 -6
Fit
Shot "n04-polaroid"

# 7. Four strings, one per color. The tying hint shows while the first is being tied.
Focus (Card "Stone Block")
Click (Find (Panel) "Tie a string" $CT::Button)
Start-Sleep -Milliseconds 600
Shot "n05-tying"
Focus (Note "Yellow")
Keys "{ENTER}"
Start-Sleep -Seconds 1
TypeText (Find (Panel) "String label" $CT::Edit) "needs"
Keys "{TAB}"
Click (Find (Panel) "Red" $CT::Button)
Shot "n06-string-selected"
Tie (Card "Oak Log") (Note "Pink") "burns" "Blue"
Tie (Note "Blue") $image "seen here" "Green"
Tie (Note "Green") (Note "White") "" "Black"
Fit

# 8. The four backgrounds with nothing selected, ending on Cork.
foreach ($background in @("Dots", "Grid", "Blank", "Cork")) {
  Background $background
  Focus (Canvas)
  Keys "{ESC}"
  Shot ("n1-background-" + $background.ToLower())
}

# 9. On cork: a note selected (pin, handles), then a card selected.
Focus (Note "Yellow")
Shot "n20-cork-note-selected"
Focus (Card "Oak Log")
Shot "n21-cork-card-selected"

# 10. Delete a note from the keyboard: its string goes too. Undo brings both back.
Focus (Note "White")
Keys "{DELETE}"
Start-Sleep -Seconds 1
Shot "n22-note-deleted"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 2
Shot "n23-note-restored"

# 11. Dim images on: the board's pictures. Then off again.
OpenSettings
Click (FindControl $win "Dim images in dark")
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 2
Shot "n24-board-dim"
OpenSettings
Click (FindControl $win "Dim images in dark")
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 2

# 12. Leave and come back.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "Clues" $CT::Hyperlink)
Start-Sleep -Seconds 2
Shot "n25-reopened"

Finish
