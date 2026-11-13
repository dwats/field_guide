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

# drive.ps1: guides, the guide page, entries in Read and Edit, images, the viewer, tags, Trash,
# the palette, the cover, and Settings.

$oak = "Oak Log $n"
$stone = "Stone Block $n"
$scrap = "Scrap $n a deliberately long entry title that keeps going well past the width of a narrow header so that wrapping can be checked in Read and in Edit"

# Scrolls the entry body to a heading: by the Contents tab, or by the heading itself.
function Jump($heading) {
  $b = Find $win $heading $CT::Button 2 -Optional
  if ($b) { Click $b } else { Reveal (Find $win $heading $CT::Text 2) }
  Start-Sleep -Milliseconds 700
}

# Opens the album from the entry header, then the viewer on the first picture.
function OpenViewer($albumShot) {
  Click (Find $win "Images" $CT::Button)
  Start-Sleep -Seconds 1
  if ($albumShot) { Shot $albumShot }
  $album = Region "Images"
  $thumb = $null
  if ($album) { $thumb = $album.FindFirst($TS::Descendants, (New-Object System.Windows.Automation.PropertyCondition($AE::ControlTypeProperty, $CT::Button))) }
  if (-not $thumb) { Miss "album thumbnail" }
  Click $thumb
  Start-Sleep -Seconds 1
}

function CloseViewer() {
  Click (Find $win "Close" $CT::Button)
  # The album may still be open under the viewer.
  if (Region "Images" 1 -Optional) { Keys "{ESC}" }
}

# 1. The guide: the one an earlier run made, or a new one.
$existing = FindPrefix $win "Minecraft" $CT::Hyperlink 3 -Optional
if ($existing) {
  Click $existing
} else {
  Click (Find $win "New guide" $CT::Button)
  TypeText (Find $win "Name" $CT::Edit) "Minecraft"
  TypeText (Find $win "Description" $CT::Edit) "Survival notes"
  Shot "01-new-guide-dialog"
  Click (Find $win "Create" $CT::Button)
}
Start-Sleep -Seconds 1
Shot "02-guide-first"

# 2. The link target, then the entry that carries every markdown feature.
NewEntry $oak
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 1
NewEntry $stone
Shot "03-entry-stub"

# 3. Edit. Paste six pictures, read their references back through the clipboard, and build
# the body from the fixture. Each picture is used once: the album does not open on an entry
# that shows one picture twice (Inbox, 2026-11-12). The sixth picture is left out of the body
# on purpose, so media cleanup has a file to list.
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
$pictures = @(
  @(320, 200, "White", "DarkOrange"), @(200, 320, "Gold", "Teal"), @(300, 180, "LightGreen", "DarkGreen"),
  @(480, 160, "LightSkyBlue", "Navy"), @(48, 48, "White", "Crimson"), @(160, 160, "Plum", "Indigo")
)
foreach ($picture in $pictures) {
  ClipImage $picture[0] $picture[1] $picture[2] $picture[3]
  Keys "^v"; Start-Sleep -Seconds 3
  Keys "{ENTER}"
}
Shot "04-edit-pasted"
Keys "^a"
Keys "^c"
$raw = [System.Windows.Forms.Clipboard]::GetText()
$refs = @([regex]::Matches($raw, 'media/[^\s)"]+') | ForEach-Object { $_.Value } | Select-Object -Unique)
Write-Host ("media references: " + ($refs -join ", "))
if ($refs.Count -lt 6) { Miss "six media references in the editor (found $($refs.Count))" }
$body = Get-Content -Raw -Encoding UTF8 (Join-Path $PSScriptRoot "fixtures\markdown.md")
$body = $body.Replace("{{OAK}}", $oak)
foreach ($i in 1..5) {
  $ref = "media/missing-$i.webp"
  if ($refs.Count -ge $i) { $ref = $refs[$i - 1] }
  $body = $body.Replace("{{IMG$i}}", $ref)
}
Keys "^a"
PasteText $body
Start-Sleep -Seconds 2
Keys "^{HOME}"
Shot "05-edit-top"
Keys "^a"
Shot "06-edit-selection"
Keys "^{END}"

# 3b. Link suggestions after [[; Enter accepts the highlighted title.
Keys "{ENTER}{ENTER}"
Keys (Esc "[[Oak")
Start-Sleep -Seconds 1
Shot "07-edit-link-suggestions"
Keys "{ENTER}"
Start-Sleep -Seconds 2
Shot "08-edit-bottom"

# 4. A nested tag from the entry page, then Read, one screenshot per section.
Click (Find $win "Add tag" $CT::Button)
TypeText (Find $win "Add tag" $CT::ComboBox) "Blocks/Natural" -Enter
Start-Sleep -Seconds 1
Click (Find $win "Read" $CT::Button)
Start-Sleep -Seconds 2
Shot "10-read-top"
$contents = Find $win "Contents" $CT::TabItem 2 -Optional
if ($contents) { Click $contents }
Jump "Lists and tasks"; Shot "11-read-lists"
Jump "A table"; Shot "12-read-table-code"
Jump "Quotes and alerts"; Shot "13-read-quotes-alerts"
Jump "Pictures"; Shot "14-read-pictures"
Jump "Footnotes and spoilers"; Shot "15-read-footnotes-spoilers"
Jump "Inline text"

# 5. A web link asks first. The script answers Cancel; it never opens the browser.
Click (Find $win "Example site" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "16-web-link-dialog"
Click (Find $win "Cancel" $CT::Button)

# 6. The album and the image viewer.
OpenViewer "17-album"
Shot "18-viewer"
Click (Find $win "Next image" $CT::Button)
Shot "19-viewer-next"
CloseViewer

# 7. A long title in Read and in Edit, the sidebar folded, the More menu, then Trash with Undo.
NewEntry $scrap
Shot "20-long-title-read"
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
Shot "21-long-title-edit"
Click (Find $win "Read" $CT::Button)
Click (Find $win "Hide sidebar" $CT::Button)
Shot "22-sidebar-folded"
Click (Find $win "Show sidebar" $CT::Button)
OpenMenu "More"
Shot "23-entry-menu"
Click (MenuItem "Move to Trash")
Start-Sleep -Seconds 1
Shot "24-guide-trash-notice"

# 8. The sidebar: a tag scope, then back to All entries.
$expand = Find $win "Expand Blocks" $CT::Button 2 -Optional
if ($expand) { Click $expand }
Click (Find $win "Natural" $CT::Button)
Start-Sleep -Seconds 1
Shot "25-sidebar-tag"
Click (Find $win "All entries" $CT::Button)

# 9. Trash, and a confirm dialog answered Cancel.
Click (FindPrefix $win "Trash" $CT::Hyperlink)
Start-Sleep -Seconds 1
Focus (Find $win "Trashed entries" $null)
Keys "{DOWN}"
Shot "26-trash"
Click (Find $win "Purge..." $CT::Button)
Start-Sleep -Milliseconds 600
Shot "27-confirm-purge"
Click (Find $win "Cancel" $CT::Button)

# 10. The Tag editor: a tag selected, a color picked from the swatches.
Click (Find $win "Tag editor" $CT::Hyperlink)
Start-Sleep -Seconds 1
Click (FindPrefix $win "Blocks" $CT::TreeItem -Contains)
Click (Find $win "Green" $CT::Button)
Start-Sleep -Milliseconds 600
Shot "28-tag-editor"

# 11. The palette: entries, then commands only.
Palette "Oak" -Stay
Shot "30-palette"
Keys "{ESC}"
Palette ">theme" -Stay
Shot "31-palette-commands"
Keys "{ESC}"

# 12. The cover, set twice. The crop box has the zoom slider. The first cover's file is left
# unused by the second.
Click (Find $win "Minecraft" $CT::Hyperlink)
Start-Sleep -Seconds 1
foreach ($round in 1..2) {
  $empty = Find $win "Set cover" $CT::Button 2 -Optional
  if ($empty) { Click $empty } else { OpenMenu "More"; Click (MenuItem "Set cover...") }
  Start-Sleep -Milliseconds 600
  if ($round -eq 1) { ClipImage 300 450 "White" "Crimson" } else { ClipImage 300 450 "LightSkyBlue" "Navy" }
  Keys "^v"
  Start-Sleep -Seconds 3
  if ($round -eq 2) { Shot "32-cover-crop" }
  Click (Find $win "Save" $CT::Button)
  Start-Sleep -Seconds 1
}
Shot "33-guide-cover"

# 13. Settings: Appearance, Editor with line numbers on, Dim images on, Media cleanup.
OpenSettings
Shot "40-settings-appearance"
Click (Find $win "Editor" $CT::Button)
Click (FindControl $win "Line numbers")
Shot "41-settings-editor"
Click (Find $win "Data" $CT::Button)
Shot "42-settings-data-backups"
Click (Find $win "Media" $CT::Button)
$cleanup = Find $win "Clean up unused media" $CT::Button
Click $cleanup
Start-Sleep -Seconds 2
Reveal $cleanup
Shot "43-settings-media-cleanup"
Click (Find $win "Appearance" $CT::Button)
Click (FindControl $win "Dim images in dark")
Shot "44-settings-dim-on"

# 14. With Dim images on: the guide page, the entry, the album, the viewer (never dimmed),
# and the editor with its gutter.
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 1
Shot "45-guide-dim"
Click (Find $win $stone $CT::Hyperlink)
Start-Sleep -Seconds 2
Jump "Pictures"
Shot "46-read-dim"
OpenViewer "46b-album-dim"
Shot "47-viewer-dim"
CloseViewer
Click (Find $win "Edit" $CT::Button)
Start-Sleep -Seconds 1
Shot "48-edit-gutter"
Click (Find $win "Read" $CT::Button)

# 15. Line numbers and Dim images back off.
OpenSettings
Click (Find $win "Editor" $CT::Button)
Click (FindControl $win "Line numbers")
Click (Find $win "Appearance" $CT::Button)
Click (FindControl $win "Dim images in dark")
Shot "49-settings-restored"

# 15b. With -Theme: the palette row Theme: System, shown in Settings, then back to the theme
# of this run. Without -Theme the saved theme is left alone.
if ($Theme) {
  Palette "Theme: System"
  Start-Sleep -Milliseconds 800
  Shot "49b-settings-theme-system"
  SetTheme $Theme
}
ScreenShot "50-screen-title-bar"

# 16. The guides list, as posters and as a table.
Click (Find $win "Back" $CT::Button)
Start-Sleep -Seconds 1
Click (Find $win "Guides" $CT::Hyperlink)
Start-Sleep -Seconds 1
Shot "51-guides-posters"
Click (Find $win "Table" $CT::Button)
Shot "52-guides-table"
Click (Find $win "Posters" $CT::Button)

# 17. A second launch exits and leaves the first window.
$second = Start-Process -FilePath $Exe -WorkingDirectory (Split-Path $Exe) -PassThru
Start-Sleep -Seconds 4
$second.Refresh()
Write-Host "second launch exited: $($second.HasExited)"
$proc.Refresh()
Write-Host "first still running: $(-not $proc.HasExited)"
if (-not $second.HasExited) { Miss "second launch did not exit"; Stop-Process -Id $second.Id -Force }

Finish
