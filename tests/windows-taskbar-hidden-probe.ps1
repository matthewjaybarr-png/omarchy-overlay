# Feasibility probe only. Hides existing Explorer taskbar windows briefly, then
# restores their visibility and the pointer. Does not change the auto-hide setting.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$results=[Collections.Generic.List[object]]::new();$bars=@();$cursor=$null
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;using System.Text;using System.Runtime.InteropServices;
public static class HiddenTaskbarProbe {
 public delegate bool EnumProc(IntPtr h,IntPtr d);
 [StructLayout(LayoutKind.Sequential)] public struct Point{public int x,y;}
 [StructLayout(LayoutKind.Sequential)] public struct Rect{public int left,top,right,bottom;}
 [StructLayout(LayoutKind.Sequential)] struct MonitorInfo{public uint size;public Rect screen,work;public uint flags;}
 [StructLayout(LayoutKind.Sequential)] struct Appbar{public uint size;public IntPtr hwnd;public uint callback,edge;public Rect rect;public IntPtr data;}
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback,IntPtr d);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder text,int n);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int show);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point p);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr h,uint flags);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern bool GetMonitorInfo(IntPtr h,ref MonitorInfo info);
 [DllImport("shell32.dll")] static extern UIntPtr SHAppBarMessage(uint message,ref Appbar data);
 public static IntPtr[] Bars(){var list=new System.Collections.Generic.List<IntPtr>();EnumWindows((h,d)=>{var t=new StringBuilder(256);GetClassName(h,t,256);if(t.ToString()=="Shell_TrayWnd"||t.ToString()=="Shell_SecondaryTrayWnd")list.Add(h);return true;},IntPtr.Zero);return list.ToArray();}
 public static uint Owner(IntPtr h){uint pid;GetWindowThreadProcessId(h,out pid);return pid;}
 public static uint AutoHide(){var a=new Appbar();a.size=(uint)Marshal.SizeOf(a);return (uint)SHAppBarMessage(4,ref a).ToUInt64()&1;}
 public static Rect Screen(IntPtr h){var m=new MonitorInfo();m.size=(uint)Marshal.SizeOf(m);if(!GetMonitorInfo(MonitorFromWindow(h,2),ref m))throw new Exception("Monitor info unavailable");return m.screen;}
}
'@
function Pump([int]$ms){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
function Check($name,[bool]$ok,$detail=''){$results.Add(@{name=$name;passed=$ok;detail=$detail});if(-not $ok){throw "$name ($detail)"}}
try {
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 Check 'Probe runs as a normal user' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
 $autoHide=[HiddenTaskbarProbe]::AutoHide()
 foreach($h in [HiddenTaskbarProbe]::Bars()){
  $ownerId=[HiddenTaskbarProbe]::Owner($h);$process=Get-Process -Id $ownerId
  Check 'Taskbar belongs to Explorer' ($process.ProcessName -eq 'explorer')
  $bars+=@{hwnd=$h;owner=$ownerId;visible=[HiddenTaskbarProbe]::IsWindowVisible($h)}
 }
 Check 'At least one taskbar is available' ($bars.Count -gt 0)
 $cursor=[HiddenTaskbarProbe+Point]::new();[void][HiddenTaskbarProbe]::GetCursorPos([ref]$cursor)
 foreach($bar in $bars){[void][HiddenTaskbarProbe]::ShowWindowAsync($bar.hwnd,0)};Pump 500
 Check 'Taskbar windows can be hidden while Explorer stays alive' (@($bars|Where-Object {[HiddenTaskbarProbe]::IsWindowVisible($_.hwnd)}).Count -eq 0)
 foreach($bar in $bars){
  $screen=[HiddenTaskbarProbe]::Screen($bar.hwnd)
  Check 'Pointer can reach the taskbar edge on the input desktop' ([HiddenTaskbarProbe]::SetCursorPos(($screen.left+($screen.right-$screen.left)/2),($screen.bottom-1)))
  $revealed=$false
  for($i=0;$i -lt 30;$i++){Pump 100;if([HiddenTaskbarProbe]::IsWindowVisible($bar.hwnd)){$revealed=$true}}
  Check 'Hidden taskbar stays hidden during three seconds at its edge' (-not $revealed)
  $ownerProcess=Get-Process -Id $bar.owner -ErrorAction SilentlyContinue
  Check 'Original Explorer process and taskbar still exist' ($null -ne $ownerProcess -and [HiddenTaskbarProbe]::IsWindow($bar.hwnd) -and [HiddenTaskbarProbe]::Owner($bar.hwnd) -eq $bar.owner)
 }
 Check 'Windows auto-hide preference was not changed' ([HiddenTaskbarProbe]::AutoHide() -eq $autoHide)
} catch {$results.Add(@{name='Hidden taskbar feasibility probe';passed=$false;detail=$_.Exception.ToString()})}
finally {
 foreach($bar in $bars){
  if([HiddenTaskbarProbe]::IsWindow($bar.hwnd) -and [HiddenTaskbarProbe]::Owner($bar.hwnd) -eq $bar.owner){[void][HiddenTaskbarProbe]::ShowWindowAsync($bar.hwnd,$(if($bar.visible){4}else{0}))}
 }
 if($null -ne $cursor){[void][HiddenTaskbarProbe]::SetCursorPos($cursor.x,$cursor.y)}
 Pump 1000
 foreach($bar in $bars){$results.Add(@{name='Original taskbar visibility restored';passed=([HiddenTaskbarProbe]::IsWindowVisible($bar.hwnd) -eq $bar.visible);detail=''})}
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 (Join-Path $root 'taskbar-hidden-probe-result.json')
}
