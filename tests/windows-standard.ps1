$ErrorActionPreference='Stop'
# Standard edition (portable, no admin): run the unzipped Standard release with a limited token and
# prove it never elevates, offers no install item and leaves nothing machine-wide behind.
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$exe=Join-Path $root 'standard\OmarchyOverlay.exe'
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;using System.Runtime.InteropServices;using System.Text;using System.Collections.Generic;
public static class Edition {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern IntPtr FindWindowEx(IntPtr p,IntPtr after,string cls,string title);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")]static extern IntPtr SendMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")]static extern int GetMenuItemCount(IntPtr m);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern int GetMenuString(IntPtr m,uint pos,StringBuilder s,int max,uint flags);
 [DllImport("user32.dll")]public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
 [DllImport("kernel32.dll")]static extern IntPtr OpenProcess(uint access,bool inherit,int pid);
 [DllImport("advapi32.dll")]static extern bool OpenProcessToken(IntPtr p,uint access,out IntPtr t);
 [DllImport("advapi32.dll")]static extern bool GetTokenInformation(IntPtr t,int cls,out int value,int len,out int ret);
 [DllImport("kernel32.dll")]static extern bool CloseHandle(IntPtr h);
 public static IntPtr MainWindow(){return FindWindow("OmarchyOverlay.Main",null);}
 public static IntPtr Overlay(){return FindWindow("OmarchyOverlay.Overlay",null);}
 // TokenElevation (20): 1 elevated, 0 not; negative when the token cannot be read.
 public static int Elevation(int pid){IntPtr p=OpenProcess(0x1000,false,pid);if(p==IntPtr.Zero)return -1;IntPtr t;if(!OpenProcessToken(p,8,out t)){CloseHandle(p);return -2;}int v,r;bool ok=GetTokenInformation(t,20,out v,4,out r);CloseHandle(t);CloseHandle(p);return ok?v:-3;}
 static IntPtr VisibleMenu(){IntPtr h=IntPtr.Zero;while((h=FindWindowEx(IntPtr.Zero,h,"#32768",null))!=IntPtr.Zero){if(IsWindowVisible(h))return h;}return IntPtr.Zero;}
 // Opens the tray menu the way a right-click does (WM_APP+1 / WM_RBUTTONUP), reads it, then cancels it.
 [DllImport("user32.dll")]static extern bool AllowSetForegroundWindow(int pid);
 public static string[] TrayItems(){
  // A real tray click gives the app foreground rights; a posted message does not, so grant them.
  keybd_event(0x10,0,0,UIntPtr.Zero);keybd_event(0x10,0,2,UIntPtr.Zero);AllowSetForegroundWindow(-1);
  PostMessage(MainWindow(),0x8001,IntPtr.Zero,(IntPtr)0x205);IntPtr popup=IntPtr.Zero;
  for(int i=0;i<60&&popup==IntPtr.Zero;i++){System.Threading.Thread.Sleep(100);popup=VisibleMenu();}
  if(popup==IntPtr.Zero)return null;
  IntPtr menu=SendMessage(popup,0x1E1,IntPtr.Zero,IntPtr.Zero);var items=new List<string>();int n=GetMenuItemCount(menu);
  for(int i=0;i<n;i++){var s=new StringBuilder(256);GetMenuString(menu,(uint)i,s,256,0x400);items.Add(s.ToString());}
  PostMessage(MainWindow(),0x1F,IntPtr.Zero,IntPtr.Zero); // WM_CANCELMODE ends the menu loop
  for(int i=0;i<20&&VisibleMenu()!=IntPtr.Zero;i++)System.Threading.Thread.Sleep(100);
  if(VisibleMenu()!=IntPtr.Zero){keybd_event(0x1b,0,0,UIntPtr.Zero);keybd_event(0x1b,0,2,UIntPtr.Zero);}
  return items.ToArray();}
}
'@
$rows=[Collections.Generic.List[object]]::new()
function Pump([int]$ms){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
function Check($name,[bool]$ok,$detail=''){$rows.Add(@{name=$name;passed=$ok;detail="$detail"});if(-not $ok){throw "$name ($detail)"}}
$machine=[ordered]@{
 'Program Files folder'={Test-Path (Join-Path $env:ProgramFiles 'OmarchyOverlay')}
 'HKLM uninstall key'={Test-Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\OmarchyOverlay'}
 'ProgramData Start menu shortcut'={Test-Path (Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\Omarchy Overlay.lnk')}
 'Scheduled task'={$null -ne (Get-ScheduledTask -TaskName 'Omarchy Overlay' -ErrorAction SilentlyContinue)}
}
$configPath=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay\config.json'
$settings=Split-Path $configPath
$original=if(Test-Path $configPath){[IO.File]::ReadAllBytes($configPath)}else{$null}
$app=$null
try{
 Check 'Standard release unzipped beside the fixture' (Test-Path $exe) $exe
 Check 'Standard release has no INSTALL.cmd or admin marker' (-not (Test-Path (Join-Path $root 'standard\INSTALL.cmd')) -and -not (Test-Path (Join-Path $root 'standard\edition-admin')))
 foreach($k in $machine.Keys){Check "Before: no $k" (-not (& $machine[$k]))}
 Check 'No existing instance to disturb' ([Edition]::MainWindow() -eq [IntPtr]::Zero)
 [IO.Directory]::CreateDirectory($settings)|Out-Null
 $session=[Diagnostics.Process]::GetCurrentProcess().SessionId
 $excluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe')+@(Get-Process|Where-Object{$_.SessionId -eq $session -and $_.MainWindowHandle -ne 0}|ForEach-Object{$_.ProcessName+'.exe'})
 [IO.File]::WriteAllText($configPath,(@{excluded_processes=@($excluded|Sort-Object -Unique);start_tiling=$false}|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
 $consentBefore=@(Get-Process consent -ErrorAction SilentlyContinue).Count
 $app=Start-Process $exe -PassThru
 $sawConsent=$false
 $until=[DateTime]::UtcNow.AddSeconds(12)
 do{Pump 150;if(@(Get-Process consent -ErrorAction SilentlyContinue).Count -gt $consentBefore){$sawConsent=$true}}while(([Edition]::MainWindow() -eq [IntPtr]::Zero -or -not [Edition]::IsWindowVisible([Edition]::Overlay())) -and -not $app.HasExited -and [DateTime]::UtcNow -lt $until)
 Check 'Standard edition starts' (-not $app.HasExited -and [Edition]::MainWindow() -ne [IntPtr]::Zero) "exited: $($app.HasExited)"
 Pump 1500
 [void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x1b,[IntPtr]::Zero);Pump 500
 $owner=Invoke-CimMethod -InputObject (Get-CimInstance Win32_Process -Filter "ProcessId=$($app.Id)") -MethodName GetOwner
 Check 'Process is owned by the signed-in user' ("$($owner.Domain)\$($owner.User)" -eq "$env:USERDOMAIN\$env:USERNAME") "$($owner.Domain)\$($owner.User)"
 $elevation=[Edition]::Elevation($app.Id)
 Check 'Process token is not elevated' ($elevation -eq 0) "TokenElevation $elevation"
 $items=[Edition]::TrayItems()
 Check 'Tray menu opens' ($null -ne $items -and $items.Count -gt 0)
 Check 'Tray menu has no install or uninstall item' (-not ($items|Where-Object{$_ -match 'Install|Uninstall'})) ($items -join ' | ')
 if(@(Get-Process consent -ErrorAction SilentlyContinue).Count -gt $consentBefore){$sawConsent=$true}
 Check 'No UAC prompt (consent.exe) appeared' (-not $sawConsent)
 [void][Edition]::PostMessage([Edition]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(10);do{Pump 150}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $until)
 Check 'Exit stops the app' $app.HasExited
 $left=@(Get-ChildItem $settings -Filter '*-restore.json' -ErrorAction SilentlyContinue|ForEach-Object Name)
 Check 'Exit restores the desktop (no recovery journals left)' ($left.Count -eq 0) ($left -join ', ')
 foreach($k in $machine.Keys){Check "After: nothing written to $k" (-not (& $machine[$k]))}
}finally{
 if($null -ne $app -and -not $app.HasExited){[void][Edition]::PostMessage([Edition]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);if(-not $app.WaitForExit(10000)){$app.Kill()}}
 if($null -ne $original){[IO.File]::WriteAllBytes($configPath,$original)}elseif(Test-Path $configPath){Remove-Item $configPath}
 $rows|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'standard-result.json')
}
