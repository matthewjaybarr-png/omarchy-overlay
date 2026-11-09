# Feasibility test, not a production tray implementation. Run as the signed-in
# normal user with Omarchy Overlay paused and taskbar Keep hidden disabled. The probe
# caches Explorer's overflow InvokePattern while visible, hides the taskbar,
# exercises a disposable real tray icon, then restores its own changes.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes,System.Windows.Forms,System.Drawing
Add-Type @'
using System;using System.Runtime.InteropServices;
public static class TrayProbe {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="FindWindowExW")] static extern IntPtr TbFindEx(IntPtr p,IntPtr a,string c,string t);
 [DllImport("user32.dll",EntryPoint="GetWindowThreadProcessId")] static extern uint TbPid(IntPtr h,out uint pid);
 // Explorer's own taskbar: Zebar makes a hidden Shell_TrayWnd that FindWindow can return first.
 public static IntPtr ExplorerTaskbar(){IntPtr first=IntPtr.Zero,h=IntPtr.Zero;while((h=TbFindEx(IntPtr.Zero,h,"Shell_TrayWnd",null))!=IntPtr.Zero){uint p;TbPid(h,out p);try{if(System.Diagnostics.Process.GetProcessById((int)p).ProcessName.Equals("explorer",System.StringComparison.OrdinalIgnoreCase))return h;}catch{}if(first==IntPtr.Zero)first=h;}return first;}
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool ShowWindowAsync(IntPtr h,int mode);
 [DllImport("user32.dll")]public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
 [DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")]public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 [DllImport("user32.dll")]public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
 public static IntPtr Popup(){return FindWindow("TopLevelWindowForOverflowXamlIsland",null);}
 public static void Key(byte k){keybd_event(k,0,0,UIntPtr.Zero);keybd_event(k,0,2,UIntPtr.Zero);}
 public static void FocusTray(){keybd_event(0x5b,0,0,UIntPtr.Zero);Key(0x42);keybd_event(0x5b,0,2,UIntPtr.Zero);}
}
'@
function Pump([int]$ms){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
$rows=[Collections.Generic.List[object]]::new()
function Check($name,[bool]$ok,$detail=''){$rows.Add(@{name=$name;passed=$ok;detail=$detail});if(-not $ok){throw "$name ($detail)"}}
$bar=[TrayProbe]::ExplorerTaskbar();$visible=[TrayProbe]::IsWindowVisible($bar);$cursor=[Windows.Forms.Cursor]::Position
$originalPopup=$null;$popup=[IntPtr]::Zero
$script:leftClicks=0
$icon=[Windows.Forms.NotifyIcon]::new();$menu=[Windows.Forms.ContextMenuStrip]::new()
try {
 [void]$menu.Items.Add('Omarchy Overlay tray probe menu')
 $icon.add_MouseClick({param($sender,$eventArgs) if($eventArgs.Button -eq [Windows.Forms.MouseButtons]::Left){$script:leftClicks++}})
 $icon.Text='Omarchy Overlay tray probe';$icon.Icon=[Drawing.SystemIcons]::Information;$icon.ContextMenuStrip=$menu;$icon.Visible=$true
 [TrayProbe]::Key(0x1b);[void][TrayProbe]::ShowWindowAsync($bar,4);[TrayProbe]::FocusTray();Pump 1400
 $element=[Windows.Automation.AutomationElement]::FromHandle($bar)
 $all=$element.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
 $target=@($all|Where-Object{$_.Current.AutomationId -eq 'SystemTrayIcon' -and $_.Current.ClassName -eq 'SystemTray.NormalButton'})
 Check 'Explorer exposes a unique native overflow button' ($target.Count -eq 1)
 $invoke=$target[0].GetCurrentPattern([Windows.Automation.InvokePattern]::Pattern)
 [void][TrayProbe]::ShowWindowAsync($bar,0);Pump 400
 Check 'Taskbar window is hidden for the invocation' (-not [TrayProbe]::IsWindowVisible($bar))
 $invoke.Invoke()
 $until=[DateTime]::UtcNow.AddSeconds(6)
 do{Pump 200;$popup=[TrayProbe]::Popup()}while(-not [TrayProbe]::IsWindowVisible($popup) -and [DateTime]::UtcNow -lt $until)
 Check 'Real overflow popup opens while the taskbar is hidden' ([TrayProbe]::IsWindowVisible($popup))
 Check 'Opening overflow did not reveal the taskbar' (-not [TrayProbe]::IsWindowVisible($bar))
 $panel=[Windows.Automation.AutomationElement]::FromHandle($popup)
 $originalPopup=$panel.Current.BoundingRectangle
 $area=[Windows.Forms.Screen]::PrimaryScreen.WorkingArea
 [void][TrayProbe]::SetWindowPos($popup,[IntPtr]::Zero,[int]($area.Right-$originalPopup.Width-20),[int]($area.Top+8),0,0,0x15);Pump 500
 $position=$panel.Current.BoundingRectangle
 Check 'Native popup can sit directly beneath the Omarchy Overlay bar' ([Math]::Abs($position.Top-($area.Top+8)) -lt 3)
 $items=$panel.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.Condition]::TrueCondition)
 $rows.Add(@{name='Observed native icons';passed=$true;detail=@($items|ForEach-Object{$_.Current.Name})})
 $fixture=@($items|Where-Object{$_.Current.Name -like '*Omarchy Overlay tray probe*'})
 Check 'The original application icon appears in the native popup' ($fixture.Count -eq 1)
 $r=$fixture[0].Current.BoundingRectangle
 [void][TrayProbe]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))
 [TrayProbe]::mouse_event(8,0,0,0,[UIntPtr]::Zero);[TrayProbe]::mouse_event(16,0,0,0,[UIntPtr]::Zero);Pump 1000
 Check 'Right-click reaches the original application menu' $menu.Visible
 Check 'Taskbar remains hidden with the original menu open' (-not [TrayProbe]::IsWindowVisible($bar))
 $screen=[Windows.Forms.SystemInformation]::VirtualScreen
 $bitmap=[Drawing.Bitmap]::new($screen.Width,$screen.Height);$graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{$graphics.CopyFromScreen($screen.Left,$screen.Top,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root 'tray-probe.png'))}finally{$graphics.Dispose();$bitmap.Dispose()}
 $menu.Close();Pump 300
 if([TrayProbe]::IsWindowVisible($popup)){$invoke.Invoke();Pump 800}
 Check 'Native popup dismisses through its own tray button' (-not [TrayProbe]::IsWindowVisible($popup))
 $invoke.Invoke();Pump 1200
 Check 'Native popup reopens while taskbar stays hidden' ([TrayProbe]::IsWindowVisible($popup) -and -not [TrayProbe]::IsWindowVisible($bar))
 $r=$fixture[0].Current.BoundingRectangle
 [void][TrayProbe]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))
 [TrayProbe]::mouse_event(2,0,0,0,[UIntPtr]::Zero);[TrayProbe]::mouse_event(4,0,0,0,[UIntPtr]::Zero);Pump 700
 Check 'Left-click reaches the original application callback' ($script:leftClicks -eq 1) $script:leftClicks
}catch{$rows.Add(@{name='Tray hidden probe';passed=$false;detail=$_.Exception.ToString()})}
finally{
 $menu.Close();[TrayProbe]::Key(0x1b)
 if($null -ne $originalPopup){[void][TrayProbe]::SetWindowPos($popup,[IntPtr]::Zero,[int]$originalPopup.Left,[int]$originalPopup.Top,0,0,0x15)}
 $icon.Visible=$false;$icon.Dispose();$menu.Dispose()
 [void][TrayProbe]::ShowWindowAsync($bar,$(if($visible){4}else{0}));[Windows.Forms.Cursor]::Position=$cursor;Pump 800
 $rows.Add(@{name='Original taskbar visibility restored';passed=([TrayProbe]::IsWindowVisible($bar) -eq $visible);detail=''})
 $rows|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'tray-hidden-result.json')
}
