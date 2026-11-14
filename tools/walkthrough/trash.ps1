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

# trash.ps1: the Trash for guides and boards. Move to Trash with Undo, the pages inside a trashed
# guide, the guides' Trash with its purge and Empty trash dialogs, a board in a guide's Trash,
# and the Vocabulary count for a relation into a trashed guide.
#
# It needs the vault that tools\walkthrough\seed writes: three guides tied to each other by
# relations, qualifier values, cards, and strings. The app has no control that makes those
# across guides. The dialog lines checked below are the counts that seed prints.

# A line of text that must be on the page. A miss is counted when it is not.
function Says($text) { $null = FindPrefix $win $text $null 3 -Contains }
# A line of text that must not be on the page.
function Unsaid($text) { if (FindPrefix $win $text $null 1 -Optional -Contains) { Miss "still shown: $text" } }

function GuidesList() {
  $link = Find $win "Guides" $CT::Hyperlink 2 -Optional
  if ($link) { Click $link }
  Start-Sleep -Seconds 1
}
function OpenGuide($name) { GuidesList; Click (FindPrefix $win $name $CT::Hyperlink); Start-Sleep -Seconds 1 }
function GuideToTrash() {
  OpenMenu "More"
  Click (MenuItem "Move guide to Trash")
  Start-Sleep -Seconds 1
}
function Back() { Click (Find $win "Back" $CT::Button); Start-Sleep -Seconds 1 }
# The Trash row of the sidebar in view: the guides list's, or a guide's.
function OpenTrash() { Click (FindPrefix $win "Trash" $CT::Hyperlink); Start-Sleep -Seconds 1 }
function SelectFirst($grid) { Focus (Find $win $grid $null); Keys "{DOWN}" }
# The confirm button of an open dialog. The page behind it can have a button of the same name.
function Confirm($dialog, $button) { Click (Find (Region $dialog) $button $CT::Button); Start-Sleep -Seconds 2 }

# 0. The seeded vault.
if (-not (FindPrefix $win "Valheim" $CT::Hyperlink 3 -Optional)) {
  Miss "the seeded guides (run tools\walkthrough\seed first; see the README)"
  Finish
}
Shot "t00-guides"

# 1. Every page inside Minecraft once, so Back can return to each: the guide page, the Tag
# editor, Vocabulary, Boards, a board, an entry, the guide's Trash.
OpenGuide "Minecraft"
Shot "t01-guide"
Click (Find $win "Tag editor" $CT::Hyperlink)
Click (Find $win "Vocabulary" $CT::Hyperlink)
Click (Find $win "Boards" $CT::Hyperlink)
Click (Find $win "Map" $CT::Hyperlink)
Start-Sleep -Seconds 2
Click (Find $win "Stone" $CT::Hyperlink)
Start-Sleep -Seconds 1
OpenTrash
Click (Find $win "Minecraft" $CT::Hyperlink)
Start-Sleep -Seconds 1

# 2. Move guide to Trash does not ask. The notice offers Undo, which opens the guide again.
OpenMenu "More"
Shot "t02-guide-menu"
Click (MenuItem "Move guide to Trash")
Start-Sleep -Seconds 1
Says 'Moved the guide "Minecraft" to Trash.'
if (FindPrefix $win "Minecraft" $CT::Hyperlink 1 -Optional) { Miss "the trashed guide is gone from the guides list" }
Shot "t03-guide-moved"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 2
Unsaid "This guide is in the Trash"
Shot "t04-guide-undo"
GuideToTrash

# 3. The palette lists no entry of the trashed guide. A new guide with its name is refused.
Palette "Furnace" -Stay
Shot "t05-palette"
Keys "{ESC}"
Click (Find $win "New guide" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Minecraft"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1
Says "is in the Trash; restore it or purge it first"
Shot "t06-guide-name-in-trash"
Click (Find $win "Cancel" $CT::Button)

# 4. Back, nine times: the guide page, the guides list, the guide page, the guide's Trash, the
# entry, the board, Boards, Vocabulary, the Tag editor. Every page inside the guide says where
# the guide is, and the guide's sidebar is not drawn.
$said = 0
foreach ($i in 1..9) {
  Back
  if (Find $win "This guide is in the Trash" $null 2 -Optional) {
    $said++
    if (Find $win "Vocabulary" $CT::Hyperlink 1 -Optional) { Miss "no guide sidebar on a page of a trashed guide (back $i)" }
  }
  Shot "t1$i-back"
}
Write-Host "pages that said the guide is in the Trash: $said of 9 (the guides list is one of the 9)"
if ($said -lt 8) { Miss "eight pages inside the trashed guide say so (found $said)" }

# 5. Restore on such a page brings the page and the guide's sidebar back.
Click (Find $win "Restore" $CT::Button)
Start-Sleep -Seconds 2
Unsaid "This guide is in the Trash"
$null = Find $win "Vocabulary" $CT::Hyperlink
Shot "t20-restored-in-place"
Click (Find $win "Minecraft" $CT::Hyperlink)
Start-Sleep -Seconds 1
GuideToTrash

# 6. Valheim while Minecraft is in the Trash. The verb "found near" is used by one relation,
# and that relation reaches into Minecraft: the count says so, and Delete is refused.
OpenGuide "Valheim"
Click (Find $win "Vocabulary" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (FindPrefix $win "found near" $CT::ListItem)
Says "Used by 1 relation. 1 of them has an entry in Trash."
Shot "t21-vocabulary-in-trash"
Click (Find $win "Delete verb" $CT::Button)
Start-Sleep -Milliseconds 600
Says "That count includes 1 relation with an entry in Trash."
Shot "t22-vocabulary-delete-refused"
Keys "{ESC}"
# The entry and the board: nothing of Minecraft shows, with Other guides on.
Click (Find $win "Flint" $CT::Hyperlink)
Start-Sleep -Seconds 2
$relations = Find $win "Relations" $CT::TabItem 2 -Optional
if ($relations) { Click $relations }
Shot "t23-entry-other-guide-hidden"
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "Camp" $CT::Hyperlink)
Start-Sleep -Seconds 2
OpenMenu "View"
Click (MenuItem "Other guides")
CloseMenu "View"
Click (Find $win "Fit the board to the window" $CT::Button)
Start-Sleep -Seconds 1
if (Find (Find $win "Board canvas" $null) "Stone" $CT::Button 1 -Optional) { Miss "no card of a trashed guide on a board of another guide" }
Shot "t24-board-other-guide-hidden"

# 7. The guides' Trash. Purge names what goes, what it takes from other guides included.
GuidesList
Shot "t30-guides-trash-row"
OpenTrash
Shot "t31-guides-trash"
SelectFirst "Trashed guides"
Click (Find $win "Purge..." $CT::Button)
Start-Sleep -Milliseconds 600
Says "4 entries, 1 of them already in Trash, are removed for good."
Says "1 tag, 1 verb, 1 qualifier label, and 6 relations go with them."
Says "4 of those relations are with entries in other guides."
Says "3 qualifiers on relations in other guides name these entries and are removed."
Says "1 board with 1 note and 2 strings is removed."
Says "3 cards come off boards in other guides."
Says "5 strings tied to those cards go with them."
Shot "t32-guide-purge-dialog"
Click (Find $win "Cancel" $CT::Button)
# Enter restores the row; Undo in the notice puts the guide back in the Trash.
SelectFirst "Trashed guides"
Keys "{ENTER}"
Start-Sleep -Seconds 1
Says 'Restored the guide "Minecraft".'
Shot "t33-guide-restored"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 1
Shot "t34-guide-restore-undone"

# 8. Two guides in the Trash. Empty trash counts them together: a relation between the two
# counts once, and neither is an "other guide".
OpenGuide "Terraria"
GuideToTrash
OpenTrash
Click (Find $win "Empty trash" $CT::Button)
Start-Sleep -Milliseconds 600
Says "2 guides are removed for good."
Says "6 entries, 1 of them already in Trash, are removed for good."
Says "1 tag, 2 verbs, 2 qualifier labels, and 7 relations go with them."
Says "2 of those relations are with entries in other guides."
Says "2 qualifiers on relations in other guides name these entries and are removed."
Says "2 boards with 2 notes and 6 strings are removed."
Says "2 cards come off boards in other guides."
Says "2 strings tied to those cards go with them."
Shot "t35-guides-empty-trash-dialog"
Confirm "Empty the Trash?" "Empty trash"
Says "Emptied the Trash: 2 guides purged."
Shot "t36-guides-trash-emptied"

# 9. What the purge left of Valheim: the relation keeps its text value, the board keeps the
# string between its own two cards, and the verb is free.
OpenGuide "Valheim"
Click (Find $win "Flint" $CT::Hyperlink)
Start-Sleep -Seconds 2
$relations = Find $win "Relations" $CT::TabItem 2 -Optional
if ($relations) { Click $relations }
Shot "t40-entry-after-purge"
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (Find $win "Camp" $CT::Hyperlink)
Start-Sleep -Seconds 2
Shot "t41-board-after-purge"
Click (Find $win "Vocabulary" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (FindPrefix $win "found near" $CT::ListItem)
Says "No relation uses this verb."
Shot "t42-vocabulary-after-purge"

# 10. A board to the Trash from the list: no dialog, Undo opens the board again.
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "t50-boards"
OpenMenu "More for Old plan"
Click (MenuItem "Move to Trash")
Start-Sleep -Seconds 1
Says 'Moved the board "Old plan" to Trash.'
Shot "t51-board-moved"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 2
Shot "t52-board-undo"
Click (Find $win "Boards" $CT::Hyperlink)
Start-Sleep -Seconds 1
OpenMenu "More for Old plan"
Click (MenuItem "Move to Trash")
Start-Sleep -Seconds 1
# A new board with the trashed name is refused.
Click (Find $win "New board" $CT::Button)
TypeText (Find $win "Name" $CT::Edit) "Old plan"
Click (Find $win "Create" $CT::Button)
Start-Sleep -Seconds 1
Says "Trash"
Shot "t53-board-name-in-trash"
Click (Find $win "Cancel" $CT::Button)
# Back to the board's page: it says where the board is. Restore brings it back.
Back
Says "This board is in the Trash"
Shot "t54-back-to-trashed-board"
Click (Find $win "Restore" $CT::Button)
Start-Sleep -Seconds 2
Unsaid "This board is in the Trash"
Shot "t55-board-restored-in-place"
# From More on the board page.
OpenMenu "More"
Click (MenuItem "Move to Trash")
Start-Sleep -Seconds 1
Shot "t56-board-moved-from-page"

# 11. The guide's Trash with one entry and one board.
NewEntry "Scrap $n"
OpenMenu "More"
Click (MenuItem "Move to Trash")
Start-Sleep -Seconds 1
Shot "t57-sidebar-trash-count"
OpenTrash
Says "2 notes, 2 strings, 1 card"
Shot "t58-guide-trash"
SelectFirst "Trashed boards"
Click (Find $win "Purge..." $CT::Button)
Start-Sleep -Milliseconds 600
Says "2 notes and 2 strings are removed for good."
Says "1 card comes off the board. The entries stay in the guide."
Shot "t59-board-purge-dialog"
Click (Find $win "Cancel" $CT::Button)
Click (Find $win "Empty trash" $CT::Button)
Start-Sleep -Milliseconds 600
Says "1 entry and 1 board are removed for good."
Says "The board holds 2 notes and 2 strings."
Says "1 card comes off the board. Entries that are not in the Trash stay in the guide."
Shot "t60-guide-empty-trash-dialog"
Click (Find $win "Cancel" $CT::Button)
# Enter restores the board; Undo puts it back; then Empty trash for real.
SelectFirst "Trashed boards"
Keys "{ENTER}"
Start-Sleep -Seconds 1
Says 'Restored the board "Old plan".'
Shot "t61-board-restored"
Click (Find $win "Undo" $CT::Button)
Start-Sleep -Seconds 1
Click (Find $win "Empty trash" $CT::Button)
Start-Sleep -Milliseconds 600
Confirm "Empty the Trash?" "Empty trash"
Says "Emptied the Trash: 1 entry and 1 board purged."
Shot "t62-guide-trash-emptied"

# 12. Two empty guides in the Trash.
foreach ($name in @("Empty one $n", "Empty two $n")) {
  GuidesList
  Click (Find $win "New guide" $CT::Button)
  TypeText (Find $win "Name" $CT::Edit) $name
  Click (Find $win "Create" $CT::Button)
  Start-Sleep -Seconds 1
  GuideToTrash
}
OpenTrash
Shot "t70-empty-guides-in-trash"
Click (Find $win "Empty trash" $CT::Button)
Start-Sleep -Milliseconds 600
Says "2 guides are removed for good."
Says "The guides are empty."
Shot "t71-empty-guides-dialog"
Confirm "Empty the Trash?" "Empty trash"
Says "Emptied the Trash: 2 guides purged."
Says "Trash is empty."
Shot "t72-trash-empty"

Finish
