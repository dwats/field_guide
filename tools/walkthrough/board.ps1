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

# board.ps1: relations, Vocabulary, the board list, a board with cards and lines, the View
# menu, the panel's Entry and Relations tabs, the four backgrounds, rename, and delete.

function Panel() { return Region "Board panel" }
function Canvas() { return Find $win "Board canvas" $null }
function Card($title) { return Find (Canvas) $title $CT::Button }
# The relation line: on the entry page, or in the board panel when a root is given.
function Line($root) { if (-not $root) { $root = $win }; return Find $root "Add relation" $CT::ComboBox }

# 1. A fresh guide, so the vocabulary starts empty.
Click (Find $win "New guide" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Boards $n"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1

# 2. Stone Block DROPS Cobblestone Block, mined with Pickaxe; the body mentions the Pickaxe.
NewEntry "Stone Block"
$relations = Find $win "Relations" $CT::TabItem 2 -Optional
if ($relations) { Click $relations }
TypeText (Line) "drops"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
TypeText (FindPrefix $win "Inverse label" $CT::Edit) "dropped by"
Click (Find $win "Add verb" $CT::Button)
TypeText (Line) "Cobblestone Block"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
Click (Find $win "Create entry" $CT::Button)
TypeText (Line) "mined with"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
TypeText (FindPrefix $win "Inverse label" $CT::Edit) "mines"
Click (Find $win "Add label" $CT::Button)
TypeText (Line) "Pickaxe"
Start-Sleep -Milliseconds 800
Click (FindPrefix $win 'Create "Pickaxe"' $CT::Button)
Start-Sleep -Milliseconds 500
$save = Find $win "Save" $CT::Button 3 -Optional
if ($save) { Click $save } else { Focus (Line); Keys "{ENTER}" }
Start-Sleep -Seconds 2
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
PasteText "Gray stone, mined with a [[Pickaxe]]."
Start-Sleep -Seconds 2
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 1
Shot "b00-entry-relations"

# 2b. Cobblestone Block gets a picture, so its card is a picture card.
Click (Find $win "Cobblestone Block" $CT::Hyperlink)
Start-Sleep -Seconds 2
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
ClipImage 240 180 "White" "SlateGray"
Keys "^v"
Start-Sleep -Seconds 3
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 1
Shot "b01-entry-picture"

# 3. Vocabulary: the verbs, one selected, then the qualifier labels.
Click (Find $win "Vocabulary" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "b02-vocabulary"
Click (FindPrefix $win "drops" $CT::ListItem)
Shot "b03-vocabulary-verb"
Click (FindPrefix $win "Qualifier labels" $CT::TabItem)
Click (FindPrefix $win "mined with" $CT::ListItem)
Shot "b04-vocabulary-label"

# 4. The board list, then a new board.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "b05-boards-list"
Click (Find $win "New board" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Overview"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 2
Shot "b06-board-empty"

# 5. One entry from the picker, then everything that matches an empty search.
$search = Find $win "Search entries to add" $CT::Edit
TypeText $search "Stone"
Start-Sleep -Seconds 1
Click (Find $win "Add Stone Block" $CT::Button)
Start-Sleep -Seconds 1
Shot "b07-one-card"
SetValue $search ""
Start-Sleep -Seconds 1
Click (FindPrefix $win "Add all" $CT::Button)
Start-Sleep -Seconds 2
Click (Find $win "Fit the board to the window" $CT::Button)
Start-Sleep -Seconds 1
Shot "b08-all-cards"

# 6. The View menu; the mention layer off and on again.
OpenMenu "View"
Shot "b09-view-menu"
Click (MenuItem "Mentions")
CloseMenu "View"
Shot "b10-no-mentions"
OpenMenu "View"
Click (MenuItem "Mentions")
CloseMenu "View"

# 7. A card selected: the panel's Entry tab, then its Relations tab, then a relation typed
# there. Its line shows on the board at once.
Focus (Card "Stone Block")
Click (Find (Panel) "Entry" $CT::TabItem)
Shot "b11-panel-entry"
Focus (Card "Pickaxe")
Click (Find (Panel) "Relations" $CT::TabItem)
Shot "b12-panel-relations"
TypeText (Line (Panel)) "drops"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
TypeText (Line (Panel)) "Cobblestone Block"
Start-Sleep -Milliseconds 600
Keys "{ENTER}"
$save = Find (Panel) "Save" $CT::Button 3 -Optional
if ($save) { Click $save } else { Focus (Line (Panel)); Keys "{ENTER}" }
Start-Sleep -Seconds 2
Shot "b13-panel-relation-added"

# 8. Keyboard on a card: nudge, remove, Undo. The notice must not cover the zoom cluster.
Focus (Card "Stone Block")
Keys "{RIGHT}{RIGHT}{RIGHT}{RIGHT}{DOWN}{DOWN}"
Start-Sleep -Seconds 1
Shot "b14-nudged"
Keys "{DELETE}"
Start-Sleep -Seconds 1
Shot "b15-removed-notice"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 1
Click (Find $win "Fit the board to the window" $CT::Button)
Start-Sleep -Seconds 1
Shot "b16-undone-fit"

# 9. The four backgrounds, ending on Dots.
foreach ($background in @("Grid", "Cork", "Blank", "Dots")) {
  OpenMenu "View"
  Click (MenuItem "Background: $background")
  CloseMenu "View"
  Start-Sleep -Milliseconds 600
  Shot ("b2-background-" + $background.ToLower())
}

# 10. Rename from More; Delete board asks first and the script answers Cancel.
OpenMenu "More"
Shot "b30-board-menu"
Click (MenuItem "Rename board...")
$name = Find $win "Name" $CT::Edit
Focus $name
Keys "^a"
TypeText $name "Overview renamed"
Click (Find $win "Rename" $CT::Button)
Start-Sleep -Seconds 1
Shot "b31-renamed"
OpenMenu "More"
Click (MenuItem "Delete board...")
Start-Sleep -Milliseconds 600
Shot "b32-confirm-delete-board"
Click (Find $win "Cancel" $CT::Button)

# 11. Leave and come back: membership, positions, pan, and zoom are restored.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "b33-boards-list-again"
Click (Find $win "Overview renamed" $CT::Hyperlink)
Start-Sleep -Seconds 2
Shot "b34-reopened"

Finish
