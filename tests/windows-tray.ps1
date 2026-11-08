$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;using System.Runtime.InteropServices;
public static class TrayProbe {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool ShowWindowAsync(IntPtr h,int mode);
 [DllImport("user32.dll")]public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
 [DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")]public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 [DllImport("user32.dll")]public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
 [StructLayout(LayoutKind.Sequential)]public struct Rect{public int left,top,right,bottom;}
 [DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")]public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")]public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern IntPtr GetProp(IntPtr h,string name);
 [DllImport("user32.dll")]public static extern void SetProcessDpiAwarenessContext(IntPtr c);
 public static IntPtr MainWindow(){return FindWindow("OmarchyOverlay.Main",null);}
 public static IntPtr Overlay() { return FindWindow("OmarchyOverlay.Overlay", null); }
 // 0.16 opens the Keyspace menu at startup; the 0.15 dashboard stays hidden until asked for.
 public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
 [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToKeyspace(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
 public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToKeyspace(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
 [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowKeyspaceWindow(IntPtr hwnd, int command);
 public static bool ShowDashboard(IntPtr main) { ShowKeyspaceWindow(main, 1); return IsWindowVisible(main); }
 public static IntPtr Bar(){return FindWindow("OmarchyOverlay.TopBar",null);}
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="FindWindowExW")] static extern IntPtr TbFindEx(IntPtr p,IntPtr a,string c,string t);
 [DllImport("user32.dll",EntryPoint="GetWindowThreadProcessId")] static extern uint TbPid(IntPtr h,out uint pid);
 // Explorer's own taskbar: Zebar makes a hidden Shell_TrayWnd that FindWindow can return first.
 public static IntPtr ExplorerTaskbar(){IntPtr first=IntPtr.Zero,h=IntPtr.Zero;while((h=TbFindEx(IntPtr.Zero,h,"Shell_TrayWnd",null))!=IntPtr.Zero){uint p;TbPid(h,out p);try{if(System.Diagnostics.Process.GetProcessById((int)p).ProcessName.Equals("explorer",System.StringComparison.OrdinalIgnoreCase))return h;}catch{}if(first==IntPtr.Zero)first=h;}return first;}
 public static IntPtr Taskbar(){return ExplorerTaskbar();}
 public static void ClickTray(){var h=Bar();Rect r;GetWindowRect(h,out r);var dpi=GetDpiForWindow(h);SetCursorPos(r.left+(int)(24*dpi/96),r.top+(int)(12*dpi/96));mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);}
 public static IntPtr Popup(){return FindWindow("TopLevelWindowForOverflowXamlIsland",null);}
 public static void Key(byte k){keybd_event(k,0,0,UIntPtr.Zero);keybd_event(k,0,2,UIntPtr.Zero);}
 public static void FocusTray(){keybd_event(0x5b,0,0,UIntPtr.Zero);Key(0x42);keybd_event(0x5b,0,2,UIntPtr.Zero);}
}
'@

function Pump([int]$ms){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
$rows=[Collections.Generic.List[object]]::new()
function Check($name,[bool]$ok,$detail=''){$rows.Add(@{name=$name;passed=$ok;detail=$detail});if(-not $ok){throw "$name ($detail)"}}
function Wait-Popup([bool]$visible){$until=[DateTime]::UtcNow.AddSeconds(8);do{Pump 150;$p=[TrayProbe]::Popup()}while([TrayProbe]::IsWindowVisible($p) -ne $visible -and [DateTime]::UtcNow -lt $until);return [TrayProbe]::IsWindowVisible($p) -eq $visible}
function Wait-Journal([bool]$exists){$until=[DateTime]::UtcNow.AddSeconds(4);do{Pump 100}while((Test-Path $journal) -ne $exists -and [DateTime]::UtcNow -lt $until);return (Test-Path $journal) -eq $exists}
function Start-App([string]$mode){
 $profile.settings.taskbar_mode=$mode
 if($null -eq $profile.settings.PSObject.Properties['start_tiling']){$profile.settings|Add-Member start_tiling $false}  # K7 default is on; this suite expects a paused start
 $profile|ConvertTo-Json -Depth 20|Set-Content -Encoding UTF8 "$folder\keyspace-profile.json"
 $script:app=Start-Process "$folder\OmarchyOverlay.exe" -PassThru
 $until=[DateTime]::UtcNow.AddSeconds(12)
 do{Pump 150}while(-not [TrayProbe]::IsWindowVisible([TrayProbe]::Bar()) -and [DateTime]::UtcNow -lt $until)
 Check "$mode controller starts paused with a visible top bar" ([TrayProbe]::IsWindowVisible([TrayProbe]::Bar()))
 Pump 2200;[TrayProbe]::CloseMenu();Pump 300
}
function Stop-App{
 [void][TrayProbe]::PostMessage([TrayProbe]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(10);do{Pump 150}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $until)
 Check 'Controller exits normally' $app.HasExited
 $script:app=$null;Pump 800
}
[TrayProbe]::SetProcessDpiAwarenessContext([IntPtr](-4))
$bar=[TrayProbe]::Taskbar();$originalVisible=[TrayProbe]::IsWindowVisible($bar);$cursor=[Windows.Forms.Cursor]::Position
$folder=Join-Path $root 'tray-case';[IO.Directory]::CreateDirectory($folder)|Out-Null
Copy-Item "$root\OmarchyOverlay.exe" "$folder\OmarchyOverlay.exe" -Force
$profile=Get-Content -Raw "$root\tray-example.json"|ConvertFrom-Json
$profile.settings.bar.left=@('system-tray');$profile.settings.bar.center=@();$profile.settings.bar.right=@('controls')
$profile.settings.bar.height=32;$profile.settings.bar.font_size=12
$profile.settings.bar.module_display= [pscustomobject]@{'system-tray'='icon-text'}
$profile.settings.show_top_bar=$true
$local=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay'
$journal=Join-Path $local 'tray-popup-restore.json'
$app=$null;$icon=[Windows.Forms.NotifyIcon]::new();$menu=[Windows.Forms.ContextMenuStrip]::new();$script:leftClicks=0
try{
 Check 'Native tray test runs without administrator privileges' (-not ([Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
 Check 'No existing controller to disturb' ([TrayProbe]::MainWindow() -eq [IntPtr]::Zero)
 [void]$menu.Items.Add('Keyspace original tray fixture menu')
 $icon.Text='Keyspace tray integration fixture';$icon.Icon=[Drawing.SystemIcons]::Information;$icon.ContextMenuStrip=$menu
 $icon.add_MouseClick({param($sender,$eventArgs) if($eventArgs.Button -eq [Windows.Forms.MouseButtons]::Left){$script:leftClicks++}})
 $icon.Visible=$true;Pump 500
 foreach($mode in @('visible','auto-hide','hidden')){
  Start-App $mode
  if($mode -eq 'hidden'){Check 'Keep hidden suppresses taskbar after tray warmup' (-not [TrayProbe]::IsWindowVisible($bar))}
  [TrayProbe]::ClickTray()
  Check "$mode Tray button opens Explorer's real popup" (Wait-Popup $true)
  Pump 700
  Check "$mode popup placement is journaled before moving" (Wait-Journal $true)
  $popup=[TrayProbe]::Popup();$pr=[TrayProbe+Rect]::new();$br=[TrayProbe+Rect]::new();[void][TrayProbe]::GetWindowRect($popup,[ref]$pr);[void][TrayProbe]::GetWindowRect([TrayProbe]::Bar(),[ref]$br)
  Check "$mode popup appears below the clicked bar" ($pr.top -ge $br.bottom -and $pr.top -lt $br.bottom+50) ("popup=$($pr.top) bar=$($br.bottom)")
  if($mode -eq 'hidden'){Check 'Keep hidden stays hidden while popup is open' (-not [TrayProbe]::IsWindowVisible($bar))}
  $panel=[Windows.Automation.AutomationElement]::FromHandle($popup)
  $items=$panel.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
  $fixture=@($items|Where-Object{$_.Current.Name -like '*Keyspace tray integration fixture*'})
  Check "$mode original fixture icon remains available" ($fixture.Count -eq 1)
  $r=$fixture[0].Current.BoundingRectangle
  [void][TrayProbe]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))
  [TrayProbe]::mouse_event(8,0,0,0,[UIntPtr]::Zero);[TrayProbe]::mouse_event(16,0,0,0,[UIntPtr]::Zero);Pump 700
  Check "$mode right-click reaches original app menu" $menu.Visible
  $menu.Close();Pump 200
  # The same visible bar button dismisses our popup and restores native placement.
  [TrayProbe]::ClickTray();Check "$mode repeated bar click dismisses popup" (Wait-Popup $false);Pump 500
  Check "$mode dismissal clears placement recovery" (Wait-Journal $false)
  Pump 700
  if($mode -eq 'hidden'){
   [TrayProbe]::keybd_event(0x11,0,0,[UIntPtr]::Zero)
   try{
    for($click=0;$click -lt 5;$click++){[TrayProbe]::ClickTray()}
    Pump 500
    Check 'Tray waits while a keyboard modifier is held' (-not [TrayProbe]::IsWindowVisible([TrayProbe]::Popup()))
   }finally{[TrayProbe]::keybd_event(0x11,0,2,[UIntPtr]::Zero)}
   Check 'Tray opens after the held modifier is released' (Wait-Popup $true)
   Pump 800
   Check 'Repeated pending clicks coalesce into one open popup' ([TrayProbe]::IsWindowVisible([TrayProbe]::Popup()))
  }else{
   [TrayProbe]::ClickTray();Check "$mode bar click reopens native popup" (Wait-Popup $true);Pump 500
  }
  Check "$mode reopened popup placement completes before interaction" (Wait-Journal $true)
  Pump 350
  $panel=[Windows.Automation.AutomationElement]::FromHandle([TrayProbe]::Popup())
  $fixture=@($panel.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)|Where-Object{$_.Current.Name -like '*Keyspace tray integration fixture*'})
  $r=$fixture[0].Current.BoundingRectangle;$before=$script:leftClicks
  [void][TrayProbe]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2));[TrayProbe]::mouse_event(2,0,0,0,[UIntPtr]::Zero);[TrayProbe]::mouse_event(4,0,0,0,[UIntPtr]::Zero);Pump 600
  Check "$mode left-click reaches original app callback" ($script:leftClicks -eq $before+1)
  if(-not [TrayProbe]::IsWindowVisible([TrayProbe]::Popup())){[TrayProbe]::ClickTray();Check "$mode popup reopens after left-click" (Wait-Popup $true);Pump 500}
  if($mode -eq 'hidden'){
   $screen=[Windows.Forms.SystemInformation]::VirtualScreen;$bitmap=[Drawing.Bitmap]::new($screen.Width,$screen.Height);$g=[Drawing.Graphics]::FromImage($bitmap)
   try{$g.CopyFromScreen($screen.Left,$screen.Top,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root 'tray-integrated.png'))}finally{$g.Dispose();$bitmap.Dispose()}
   $app.Kill();[void]$app.WaitForExit(5000);$app=$null
   $until=[DateTime]::UtcNow.AddSeconds(12);do{Pump 200}while((Test-Path $journal) -and [DateTime]::UtcNow -lt $until)
   Check 'Recovery guard clears tray popup placement after controller termination' (-not (Test-Path $journal))
   Check 'Recovery guard dismisses the owned popup' (-not [TrayProbe]::IsWindowVisible([TrayProbe]::Popup()))
   Check 'Recovery guard restores original taskbar visibility' ([TrayProbe]::IsWindowVisible($bar) -eq $originalVisible)
  }else{
   Stop-App
   Check "$mode normal exit dismisses the owned popup" (-not [TrayProbe]::IsWindowVisible([TrayProbe]::Popup()))
   Check "$mode normal exit clears the popup recovery journal" (-not (Test-Path $journal))
  }
 }
}catch{$rows.Add(@{name='Native tray integration harness';passed=$false;detail=$_.Exception.ToString()})}
finally{
 if($null -ne $app -and -not $app.HasExited){Stop-App}
 $menu.Close();$icon.Visible=$false;$icon.Dispose();$menu.Dispose();[Windows.Forms.Cursor]::Position=$cursor
 $rows|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'tray-result.json')
}
