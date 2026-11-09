$ErrorActionPreference='Stop'
# Admin edition: run INSTALL.cmd from the unzipped Admin release with the full admin token, check the
# install, that bound keys reach the app over an elevated window, that the terminal starts
# unelevated, then uninstall and check nothing is left. Runs with --elevated (RunLevel Highest).
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$admin=Join-Path $root 'admin'
$dir=Join-Path $env:ProgramFiles 'OmarchyOverlay'
$installed=Join-Path $dir 'OmarchyOverlay.exe'
$key='HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\OmarchyOverlay'
$shortcut=Join-Path $env:ProgramData 'Microsoft\Windows\Start Menu\Programs\Omarchy Overlay.lnk'
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
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern int GetDlgItemText(IntPtr h,int id,StringBuilder s,int max);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]static extern int GetWindowText(IntPtr h,StringBuilder s,int max);
 [DllImport("user32.dll")]static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")]static extern IntPtr GetForegroundWindow();
 // A Shift tap makes this process the last input source, so Windows lets it take the foreground.
 public static bool Focus(IntPtr h){keybd_event(0x10,0,0,UIntPtr.Zero);keybd_event(0x10,0,2,UIntPtr.Zero);SetForegroundWindow(h);System.Threading.Thread.Sleep(200);return GetForegroundWindow()==h;}
 // The installer's MessageBox: a visible #32770 dialog titled "Omarchy Overlay".
 public static IntPtr Box(){IntPtr h=IntPtr.Zero;while((h=FindWindowEx(IntPtr.Zero,h,"#32770",null))!=IntPtr.Zero){var t=new StringBuilder(64);GetWindowText(h,t,64);if(IsWindowVisible(h)&&t.ToString()=="Omarchy Overlay")return h;}return IntPtr.Zero;}
 public static string BoxText(IntPtr h){var s=new StringBuilder(2048);GetDlgItemText(h,0xFFFF,s,2048);return s.ToString();}
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
function Task{Get-ScheduledTask -TaskName 'Omarchy Overlay' -ErrorAction SilentlyContinue}
function Installed-App{Get-Process OmarchyOverlay -ErrorAction SilentlyContinue|Where-Object{$_.Path -eq $installed}}
# Runs a command that ends in the installer's MessageBox; returns the box text after pressing OK.
# -Hold leaves the box open that many seconds first, as a person reading it would.
function Run-Boxed([string]$file,[string]$arguments,[int]$Hold=0){
 $p=Start-Process $file $arguments -PassThru
 $until=[DateTime]::UtcNow.AddSeconds(60);do{Pump 200;$box=[Edition]::Box()}while($box -eq [IntPtr]::Zero -and -not $p.HasExited -and [DateTime]::UtcNow -lt $until)
 if($box -eq [IntPtr]::Zero){return "(no message box; exited $($p.HasExited))"}
 $text=[Edition]::BoxText($box)
 Pump ($Hold*1000)
 [void][Edition]::PostMessage($box,0x10,[IntPtr]::Zero,[IntPtr]::Zero) # WM_CLOSE: a posted IDOK does not close this box
 [void]$p.WaitForExit(15000)
 return $text
}
# The key hook ignores injected input, so the host runner presses these on the VM's keyboard.
function Real-Keys([string]$keys){$request=Join-Path $root 'keys-request.txt';Set-Content -Encoding ASCII $request $keys
 $until=[DateTime]::UtcNow.AddSeconds(30);do{Pump 200}while((Test-Path $request) -and [DateTime]::UtcNow -lt $until);Pump 1000}
function Gone{-not (Task) -and -not (Test-Path $key) -and -not (Test-Path $shortcut) -and -not (Test-Path $dir)}
function Wait-Gone{$until=[DateTime]::UtcNow.AddSeconds(15);do{Pump 300}while(-not (Gone) -and [DateTime]::UtcNow -lt $until);Gone}
$configPath=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay\config.json'
$settings=Split-Path $configPath
$original=$null;$form=$null;$spawned=@()
try{
 $id=[Security.Principal.WindowsIdentity]::GetCurrent()
 Check 'Fixture runs with the full admin token' ([Security.Principal.WindowsPrincipal]::new($id).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) $id.Name
 if(Test-Path $installed){[void](Run-Boxed $installed '--uninstall');[void](Wait-Gone)} # a failed earlier run
 Check 'Starts with no install present' (Gone)
 Check 'Admin release unzipped with INSTALL.cmd and marker' ((Test-Path (Join-Path $admin 'INSTALL.cmd')) -and (Test-Path (Join-Path $admin 'edition-admin')))
 $original=if(Test-Path $configPath){[IO.File]::ReadAllBytes($configPath)}else{$null}
 [IO.Directory]::CreateDirectory($settings)|Out-Null
 $session=[Diagnostics.Process]::GetCurrentProcess().SessionId
 $excluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe','powershell.exe')+@(Get-Process|Where-Object{$_.SessionId -eq $session -and $_.MainWindowHandle -ne 0}|ForEach-Object{$_.ProcessName+'.exe'})
 [IO.File]::WriteAllText($configPath,(@{excluded_processes=@($excluded|Sort-Object -Unique);start_tiling=$false}|ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))

 # The unzipped Admin copy offers "Install (admin)…" in its tray before installing.
 $portable=Start-Process (Join-Path $admin 'OmarchyOverlay.exe') -PassThru
 $until=[DateTime]::UtcNow.AddSeconds(12);do{Pump 150}while([Edition]::MainWindow() -eq [IntPtr]::Zero -and -not $portable.HasExited -and [DateTime]::UtcNow -lt $until)
 Pump 1500;[void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x1b,[IntPtr]::Zero);Pump 400
 $items=[Edition]::TrayItems()
 Check 'Unzipped Admin copy offers Install (admin) in the tray' (($items|Where-Object{$_ -like 'Install (admin)*'}) -and -not ($items|Where-Object{$_ -like 'Uninstall*'})) ($items -join ' | ')

 # INSTALL.cmd closes that copy, installs, starts the logon task and reports.
 $text=Run-Boxed $env:ComSpec ('/c "'+(Join-Path $admin 'INSTALL.cmd')+'"') -Hold 12
 Check 'INSTALL.cmd reports success' ($text -like 'Omarchy Overlay is installed in*') $text
 Check 'Running unzipped copy was closed by the installer' $portable.HasExited
 Check 'Program files copied' ((Test-Path $installed) -and (Test-Path (Join-Path $dir 'RECOVER.cmd')) -and @(Get-ChildItem (Join-Path $dir 'fonts') -File -ErrorAction SilentlyContinue).Count -gt 0)
 $task=Task
 Check 'Logon task runs highest for the installing user' ($null -ne $task -and "$($task.Principal.RunLevel)" -eq 'Highest' -and $task.Principal.UserId -like "*$env:USERNAME") "$($task.Principal.RunLevel) $($task.Principal.UserId)"
 Check 'Logon task starts the installed exe' ($task.Actions[0].Execute.Trim('"') -eq $installed) $task.Actions[0].Execute
 $set=$task.Settings
 Check 'Logon task has no time limit and runs on battery' ($set.ExecutionTimeLimit -eq 'PT0S' -and -not $set.DisallowStartIfOnBatteries -and -not $set.StopIfGoingOnBatteries) "limit $($set.ExecutionTimeLimit) disallow $($set.DisallowStartIfOnBatteries) stop $($set.StopIfGoingOnBatteries)"
 $link=(New-Object -ComObject WScript.Shell).CreateShortcut($shortcut)
 Check 'Start menu shortcut runs the task' ((Test-Path $shortcut) -and $link.TargetPath -like '*schtasks.exe' -and $link.Arguments -like '*/Run*Omarchy Overlay*') "$($link.TargetPath) $($link.Arguments)"
 $values=Get-ItemProperty $key -ErrorAction SilentlyContinue
 Check 'Apps entry registered' ($null -ne $values -and $values.DisplayName -eq 'Omarchy Overlay' -and $values.UninstallString -like '*--uninstall' -and $values.NoModify -eq 1) "$($values.DisplayName) $($values.UninstallString)"

 $until=[DateTime]::UtcNow.AddSeconds(15);do{Pump 300;$app=Installed-App}while(-not $app -and [DateTime]::UtcNow -lt $until)
 Check 'Installed copy is running' ($null -ne $app)
 $app=@($app)[0]
 $elevation=[Edition]::Elevation($app.Id)
 Check 'Installed copy runs elevated' ($elevation -eq 1) "TokenElevation $elevation"
 $until=[DateTime]::UtcNow.AddSeconds(10);do{Pump 200}while([Edition]::MainWindow() -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $until)
 # The task starts it while the install box is still open; it must not stall on the recovery lock.
 Check 'Installed copy started while the install box was open' ([Edition]::MainWindow() -ne [IntPtr]::Zero) $(if([Edition]::Box() -ne [IntPtr]::Zero){[Edition]::BoxText([Edition]::Box())})
 Pump 1500;[void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x1b,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(4);do{Pump 150}while([Edition]::IsWindowVisible([Edition]::Overlay()) -and [DateTime]::UtcNow -lt $until)
 $items=[Edition]::TrayItems()
 Check 'Installed copy offers Uninstall, not Install' (($items|Where-Object{$_ -like 'Uninstall*'}) -and -not ($items|Where-Object{$_ -like 'Install (admin)*'})) $(if($null -eq $items){'tray menu did not open'}else{$items -join ' | '})

 # An elevated window has focus: this fixture runs elevated, so its own form is one.
 $form=[Windows.Forms.Form]::new();$form.Text='Elevated focus window';$form.SetBounds(200,200,600,400);$form.Show();Pump 300
 Check 'Elevated window is in focus' ([Edition]::Focus($form.Handle))
 Real-Keys 'KEY_LEFTMETA KEY_K' # Super+K: keybindings
 $until=[DateTime]::UtcNow.AddSeconds(5);do{Pump 150}while(-not [Edition]::IsWindowVisible([Edition]::Overlay()) -and [DateTime]::UtcNow -lt $until)
 Check 'Super+K over an elevated window opens Keybindings' ([Edition]::IsWindowVisible([Edition]::Overlay()))
 [void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x1b,[IntPtr]::Zero);Pump 600
 Check 'Elevated window is back in focus' ([Edition]::Focus($form.Handle))
 $names='WindowsTerminal','wt','cmd','OpenConsole'
 $before=@(Get-Process $names -ErrorAction SilentlyContinue|ForEach-Object Id)
 Real-Keys 'KEY_LEFTMETA KEY_ENTER' # Super+Return: terminal
 $until=[DateTime]::UtcNow.AddSeconds(10)
 do{Pump 300;$spawned=@(Get-Process $names -ErrorAction SilentlyContinue|Where-Object{$before -notcontains $_.Id})}while(-not ($spawned|Where-Object{-not $_.HasExited}) -and [DateTime]::UtcNow -lt $until)
 $live=@($spawned|Where-Object{-not $_.HasExited})
 Check 'Super+Return starts a terminal' ($live.Count -gt 0) (($spawned|ForEach-Object{$_.ProcessName}) -join ',')
 $levels=@($live|ForEach-Object{"$($_.ProcessName)=$([Edition]::Elevation($_.Id))"})
 Check 'Terminal started from the elevated app is not elevated' (-not ($levels|Where-Object{$_ -notlike '*=0'})) ($levels -join ', ')
 $spawned|Where-Object{-not $_.HasExited}|Stop-Process -Force -ErrorAction SilentlyContinue;$spawned=@()
 $form.Close();$form=$null

 # Menu > Update > Process > Restart: the new copy must stay elevated.
 Real-Keys 'KEY_LEFTMETA KEY_LEFTALT KEY_SPACE'
 $until=[DateTime]::UtcNow.AddSeconds(5);do{Pump 150}while(-not [Edition]::IsWindowVisible([Edition]::Overlay()) -and [DateTime]::UtcNow -lt $until)
 foreach($label in 'Update','Process','Restart Omarchy'){foreach($c in $label.ToCharArray()){[void][Edition]::PostMessage([Edition]::Overlay(),0x102,[IntPtr][int]$c,[IntPtr]::Zero)};Pump 300;[void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x0d,[IntPtr]::Zero);Pump 600}
 $old=$app.Id
 $until=[DateTime]::UtcNow.AddSeconds(20);do{Pump 300;$restarted=@(Installed-App|Where-Object{$_.Id -ne $old})}while(($restarted.Count -eq 0 -or (Get-Process -Id $old -ErrorAction SilentlyContinue)) -and [DateTime]::UtcNow -lt $until)
 Check 'Restart from the menu starts a new installed copy' ($restarted.Count -gt 0 -and -not (Get-Process -Id $old -ErrorAction SilentlyContinue)) "new $($restarted.Count) old alive $([bool](Get-Process -Id $old -ErrorAction SilentlyContinue))"
 $elevation=[Edition]::Elevation($restarted[0].Id)
 Check 'Restarted installed copy is still elevated' ($elevation -eq 1) "TokenElevation $elevation"
 $until=[DateTime]::UtcNow.AddSeconds(10);do{Pump 200}while([Edition]::MainWindow() -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $until)
 Pump 1500;[void][Edition]::PostMessage([Edition]::Overlay(),0x100,[IntPtr]0x1b,[IntPtr]::Zero);Pump 500

 $text=Run-Boxed $installed '--uninstall'
 Check 'Uninstall reports the desktop restored' ($text -eq 'Omarchy Overlay is uninstalled and the desktop is restored.') $text
 Check 'Installed copy was stopped' (-not (Installed-App))
 Check 'Task, shortcut, Apps entry and folder are removed' (Wait-Gone) "task $([bool](Task)) key $(Test-Path $key) link $(Test-Path $shortcut) dir $(Test-Path $dir)"
 $left=@(Get-ChildItem $settings -Filter '*-restore.json' -ErrorAction SilentlyContinue|ForEach-Object Name)
 Check 'Recovery is clean after uninstall (no journals left)' ($left.Count -eq 0) ($left -join ', ')
}finally{
 if($null -ne $form){$form.Close()}
 $spawned|Where-Object{-not $_.HasExited}|Stop-Process -Force -ErrorAction SilentlyContinue
 Get-Process OmarchyOverlay -ErrorAction SilentlyContinue|Where-Object{$_.Path -like "$admin*"}|ForEach-Object{[void][Edition]::PostMessage([Edition]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);if(-not $_.WaitForExit(10000)){$_.Kill()}}
 if(Test-Path $installed){try{[void](Run-Boxed $installed '--uninstall');[void](Wait-Gone)}catch{}}
 # A stalled copy or uninstaller must not outlive the fixture and block the next run.
 Get-Process OmarchyOverlay -ErrorAction SilentlyContinue|Where-Object{$_.Path -eq $installed}|Stop-Process -Force -ErrorAction SilentlyContinue
 if($null -ne $original){[IO.File]::WriteAllBytes($configPath,$original)}elseif(Test-Path $configPath){Remove-Item $configPath}
 $rows|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'install-result.json')
}
