#!/usr/bin/env python3
"""Copy test artifacts and run commands through an existing QEMU guest agent.

The guest agent runs as SYSTEM. Use 'interactive' for GUI tests so the app
runs with the logged-in user's normal, limited token instead.
"""
import argparse
import base64
import json
import os
import pathlib
import subprocess
import time


def agent(command, arguments=None):
    payload = {"execute": command}
    if arguments is not None:
        payload["arguments"] = arguments
    result = subprocess.run(
        ["virsh", "-c", "qemu:///system", "qemu-agent-command", OPTIONS.vm, json.dumps(payload)],
        text=True, capture_output=True, check=True,
    )
    result = json.loads(result.stdout)
    if "error" in result:
        raise RuntimeError(result["error"])
    return result["return"]


def powershell(source):
    encoded = base64.b64encode(source.encode("utf-16-le")).decode()
    pid = agent("guest-exec", {
        "path": r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe",
        "arg": ["-NoProfile", "-NonInteractive", "-EncodedCommand", encoded], "capture-output": True,
    })["pid"]
    deadline = time.monotonic() + 55
    while time.monotonic() < deadline:
        status = agent("guest-exec-status", {"pid": pid})
        if status.get("exited"):
            stdout = base64.b64decode(status.get("out-data", "")).decode("utf-8", "replace")
            stderr = base64.b64decode(status.get("err-data", "")).decode("utf-8", "replace")
            if status.get("exitcode", 1):
                raise RuntimeError(f"Guest exit {status.get('exitcode')}: {stderr}\n{stdout}")
            return stdout
        time.sleep(0.3)
    raise TimeoutError(f"Guest command still running (pid {pid})")


def quote(value):
    return "'" + str(value).replace("'", "''") + "'"


def push(local, remote):
    handle = agent("guest-file-open", {"path": remote, "mode": "wb"})
    try:
        with open(local, "rb") as source:
            while block := source.read(65536):
                written = agent("guest-file-write", {"handle": handle, "buf-b64": base64.b64encode(block).decode()})
                if written["count"] != len(block):
                    raise IOError("Short write to guest")
    finally:
        agent("guest-file-close", {"handle": handle})
    print(f"Copied {local} to {remote}")


def pull(remote, local):
    handle = agent("guest-file-open", {"path": remote, "mode": "rb"})
    try:
        with open(local, "wb") as destination:
            while True:
                result = agent("guest-file-read", {"handle": handle, "count": 65536})
                destination.write(base64.b64decode(result.get("buf-b64", "")))
                if result.get("eof"):
                    break
    finally:
        agent("guest-file-close", {"handle": handle})
    print(f"Copied {remote} to {local}")


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--vm", default=os.environ.get("OVERLAY_VM", "Windows 11 Test"))
commands = parser.add_subparsers(dest="command", required=True)
for name in ["push", "pull"]:
    p = commands.add_parser(name)
    p.add_argument("source")
    p.add_argument("destination")
p = commands.add_parser("ps")
p.add_argument("source", help="PowerShell expression, or @path to read a script")
p = commands.add_parser("interactive")
p.add_argument("script", help="Absolute guest path to a PowerShell script")
p.add_argument("--user", default=os.environ.get("OVERLAY_VM_USER", "tester"))
p.add_argument("--elevated", action="store_true", help="Run with the user's full admin token (RunLevel Highest)")
p = commands.add_parser("keys", help="Press keys on the VM's virtual keyboard (not flagged as injected)")
p.add_argument("keys", nargs="+", help="Linux key names pressed together, e.g. KEY_LEFTMETA KEY_K")
OPTIONS = parser.parse_args()
if OPTIONS.command == "push":
    push(OPTIONS.source, OPTIONS.destination)
elif OPTIONS.command == "pull":
    pull(OPTIONS.source, OPTIONS.destination)
elif OPTIONS.command == "ps":
    source = OPTIONS.source
    if source.startswith("@"):
        source = pathlib.Path(source[1:]).read_text()
    print(powershell(source))
elif OPTIONS.command == "keys":
    subprocess.run(["virsh", "-c", "qemu:///system", "send-key", OPTIONS.vm, *OPTIONS.keys], check=True,
                   capture_output=True)
    print("Pressed " + "+".join(OPTIONS.keys))
elif OPTIONS.command == "interactive":
    task = f"Overlay-Test-{int(time.time())}-{os.getpid()}"
    wrapped = f"""
$ErrorActionPreference = 'Stop'
Start-Transcript -Path {quote(OPTIONS.script + '.console.log')} -Force | Out-Null
try {{ & {quote(OPTIONS.script)} }}
catch {{ Write-Output ($_ | Out-String); exit 1 }}
finally {{ Stop-Transcript | Out-Null }}
"""
    encoded = base64.b64encode(wrapped.encode('utf-16-le')).decode()
    source = f"""
$ErrorActionPreference = 'Stop'
$action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument '-NoProfile -ExecutionPolicy Bypass -EncodedCommand {encoded}'
$principal = New-ScheduledTaskPrincipal -UserId {quote(OPTIONS.user)} -LogonType Interactive -RunLevel {'Highest' if OPTIONS.elevated else 'Limited'}
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 5)
Register-ScheduledTask -TaskName {quote(task)} -Action $action -Principal $principal -Settings $settings | Out-Null
Start-ScheduledTask -TaskName {quote(task)}
"Started {'elevated' if OPTIONS.elevated else 'limited'} interactive task: {task}"
"""
    print(powershell(source))
