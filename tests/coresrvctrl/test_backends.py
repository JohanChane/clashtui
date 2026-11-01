"""Exercise shipped scripts against fake managers; never touch real services."""
import json
import os
import shlex
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2] / "contrib/scripts/coresrvctrl"
FAKE = r'''#!PYTHON
import json, os, pathlib, sys
path = pathlib.Path(os.environ['FAKE_STATE'])
s = json.loads(path.read_text())
tool = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
with open(os.environ['FAKE_LOG'], 'a') as log: log.write(json.dumps([tool] + args) + '\n')
if tool == 'id':
 assert args == ['-u'], args
 print(os.environ['FAKE_UID']); sys.exit(0)
if tool == 'sudo':
 if s.get('sudo_error'):
  print('sudo: authentication denied', file=sys.stderr); sys.exit(23)
 args = args[1:] if args[:1] == ['-n'] else args
 assert args[:1] == ['--'], args
 args = args[1:]
 assert args[0] in ('systemctl', 'rc-service', 'rc-update', 'launchctl'), args
 os.execv(str(pathlib.Path(sys.argv[0]).with_name(args[0])), args)
args = [a for a in args if a not in ('--user', '--')]
def save(): path.write_text(json.dumps(s))
def error(text='Permission denied', code=1):
 print(text, file=sys.stderr); sys.exit(code)
if tool == 'systemctl':
 if args[0] == 'show':
  if s.get('query_error'): error()
  print('LoadState=' + ('not-found' if s.get('missing') else 'loaded'))
  print('ActiveState=' + s.get('state', 'inactive'))
  sys.exit(4 if s.get('missing') else 0)
 if s.get('write_error'): error()
elif tool == 'rc-service':
 if args == ['--help']:
  print('--user' if not s.get('old_openrc') else 'old help'); sys.exit(0)
 if args[0] == '--exists':
  if s.get('query_error'): error()
  sys.exit(1 if s.get('missing') else 0)
 if args[-1] == 'status':
  if s.get('query_error'): error()
  state = s.get('state', 'inactive')
  text, code = {'active':('started',0), 'inactive':('stopped',3), 'crashed':('crashed',32), 'activating':('starting',8)}[state]
  print(' * status: ' + text); sys.exit(code)
 if s.get('write_error'): error()
elif tool == 'rc-update':
 if args[0] == 'show':
  if s.get('membership_error'): error()
  if s.get('enabled'): print(' svc | default')
  sys.exit(0)
 if s.get('write_error'): error()
 if args[0] == 'del' and not s.get('enabled'): error('not in runlevel')
 s['enabled'] = args[0] == 'add'; save(); sys.exit(0)
elif tool == 'launchctl':
 if args[0] == 'print':
  if s.get('query_error'): error()
  if args[1] == 'system' or (args[1].startswith('gui/') and args[1].count('/') == 1):
   if s.get('domain_error'): error()
   print('domain'); sys.exit(0)
  if s.get('loaded'): print('state = ' + ('running' if s.get('state') == 'active' else 'waiting')); sys.exit(0)
  error('Could not find service "svc" in domain',113)
 if s.get('write_error'): error()
 if args[0] == 'load':
  if (not s.get('enabled',True) and '-F' not in args) or s.get('silent_load_failure'): sys.exit(0)
  s['loaded']=True; s['state']='active'
 if args[0] == 'unload':
  if not s.get('silent_unload_failure'): s['loaded']=False; s['state']='inactive'
 if args[0] in ('enable','disable'): s['enabled']=args[0]=='enable'
 save(); sys.exit(0)
if args[-1] in ('start','restart') or args[0] in ('start','restart'): s['state']='active'
if args[-1] == 'stop' or args[0] == 'stop': s['state']='inactive'
save()
'''


@unittest.skipIf(os.name == "nt", "POSIX shell scripts run on Unix CI")
class BackendTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="clashtui script 中文 ")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.bin = self.base / "bin"
        self.bin.mkdir()
        self.state = self.base / "state.json"
        self.log = self.base / "calls.jsonl"
        self.log.touch()
        for tool in ("systemctl", "rc-service", "rc-update", "launchctl", "sudo", "id"):
            path = self.bin / tool
            path.write_text(FAKE.replace("#!PYTHON", "#!" + sys.executable))
            path.chmod(0o755)
        agents = self.base / "Library/LaunchAgents"
        agents.mkdir(parents=True)
        self.plist = agents / "svc.plist"
        self.plist.write_text("<plist/>")
        daemons = self.base / "Library/LaunchDaemons"
        daemons.mkdir()
        self.system_plist = daemons / "svc.plist"
        self.system_plist.write_text("<plist/>")
        # Relocate only the system plist directory; never inspect host services.
        self.launchd_script = self.base / "launchd-service.sh"
        self.launchd_script.write_text((ROOT / "launchd/service.sh").read_text().replace(
            "directory=/Library/LaunchDaemons", "directory=" + shlex.quote(str(daemons))))
        self.env = dict(os.environ, PATH=str(self.bin) + os.pathsep + os.environ["PATH"],
                        HOME=str(self.base), XDG_RUNTIME_DIR=str(self.base),
                        FAKE_STATE=str(self.state), FAKE_LOG=str(self.log), FAKE_UID="1000")
        self.set_state()

    def set_state(self, **values):
        self.state.write_text(json.dumps(dict(state="inactive", **values)))
        self.log.write_text("")

    def run_script(self, backend, operation, scope="user", service="svc", trace=False):
        return subprocess.run(["/bin/sh", *(["-x"] if trace else []),
                               str(self.launchd_script if backend == "launchd" else ROOT / backend / "service.sh"), operation, scope, service],
                              env=self.env, capture_output=True, text=True, timeout=10)

    def calls(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_states_and_user_scope(self):
        for backend in ("systemd", "openrc", "launchd"):
            for state in ("active", "inactive"):
                with self.subTest(backend=backend, state=state):
                    self.state.write_text(json.dumps({"state": state, "loaded": True}))
                    result = self.run_script(backend, "status")
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(result.stdout, state + "\n")
            if backend != "launchd":
                self.assertTrue(all("--user" in call for call in self.calls() if call[1:] != ["--help"]))
            else:
                self.assertIn("gui/" + self.env["FAKE_UID"] + "/svc", self.calls()[-1])
            self.log.write_text("")

    def test_missing_is_successful_status_and_stop(self):
        self.plist.unlink()
        for backend in ("systemd", "openrc", "launchd"):
            self.set_state(missing=True)
            result = self.run_script(backend, "status")
            self.assertEqual((result.returncode, result.stdout), (0, "missing\n"), result.stderr)
            self.assertEqual(self.run_script(backend, "stop").returncode, 0)
            for op in ("start", "restart", "enable-autostart", "disable-autostart"):
                self.assertNotEqual(self.run_script(backend, op).returncode, 0)

    def test_system_scope_never_becomes_user_scope(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.state.write_text(json.dumps({"state": "active", "loaded": True}))
            self.log.write_text("")
            result = self.run_script(backend, "status", scope="system")
            self.assertEqual((result.returncode, result.stdout), (0, "active\n"), result.stderr)
            self.assertFalse(any("--user" in call for call in self.calls()))
            if backend == "launchd": self.assertEqual(self.calls()[-1], ["launchctl", "print", "system/svc"])

    def test_query_failure_is_not_missing(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.set_state(query_error=True)
            result = self.run_script(backend, "status")
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertNotEqual(self.run_script(backend, "stop").returncode, 0)

    def test_start_is_idempotent_and_restart_is_not_skipped(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.state.write_text(json.dumps({"state": "active", "loaded": True}))
            self.log.write_text("")
            self.assertEqual(self.run_script(backend, "start").returncode, 0)
            calls = self.calls()
            self.assertFalse(any(call[1] in ("start", "load", "unload") or call[-1] == "start" for call in calls))
            result = self.run_script(backend, "restart")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(any("restart" in call or "load" in call for call in self.calls()))

    def test_write_failure_propagates(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.set_state(write_error=True)
            self.assertNotEqual(self.run_script(backend, "start").returncode, 0)

    def test_autostart_does_not_start_or_stop(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.set_state(enabled=True, loaded=True)
            for op in ("enable-autostart", "enable-autostart", "disable-autostart", "disable-autostart"):
                result = self.run_script(backend, op)
                self.assertEqual(result.returncode, 0, result.stderr)
            for call in self.calls():
                self.assertFalse(any(arg in ("start", "stop", "restart", "load", "unload", "--now", "mask") for arg in call))
            if backend == "openrc":
                self.assertEqual(sum("del" in call for call in self.calls()), 1)

    def test_openrc_requires_user_runtime_and_supported_version(self):
        self.env.pop("XDG_RUNTIME_DIR")
        self.assertNotEqual(self.run_script("openrc", "status").returncode, 0)
        self.env["XDG_RUNTIME_DIR"] = str(self.base)
        self.set_state(old_openrc=True)
        self.assertNotEqual(self.run_script("openrc", "status").returncode, 0)

    def test_openrc_membership_failure_is_not_already_disabled(self):
        self.set_state(membership_error=True)
        result = self.run_script("openrc", "disable-autostart")
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any("del" in call for call in self.calls()))

    def test_missing_tool_is_query_failure(self):
        for backend, tool in (("systemd", "systemctl"), ("openrc", "rc-service"), ("launchd", "launchctl")):
            path = self.bin / tool
            path.rename(self.bin / (tool + ".hidden"))
            previous = self.env["PATH"]
            self.env["PATH"] = str(self.bin)
            try:
                result = self.run_script(backend, "status")
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")
            finally:
                self.env["PATH"] = previous
                (self.bin / (tool + ".hidden")).rename(path)

    def test_transition_is_not_already_stopped(self):
        for backend in ("systemd", "openrc"):
            self.state.write_text(json.dumps({"state": "activating"}))
            self.log.write_text("")
            result = self.run_script(backend, "stop")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(any("stop" in call for call in self.calls()))

    def test_launchd_verifies_legacy_exit_zero(self):
        self.set_state(silent_load_failure=True)
        self.assertNotEqual(self.run_script("launchd", "start").returncode, 0)
        self.set_state(loaded=True, silent_unload_failure=True)
        self.assertNotEqual(self.run_script("launchd", "restart").returncode, 0)
        self.assertFalse(any("load" in call for call in self.calls()))

    def test_launchd_manual_start_and_restart_preserve_disabled_autostart(self):
        for scope in ("system", "user"):
            self.set_state(enabled=False)
            for op in ("start", "stop", "start", "restart"):
                result = self.run_script("launchd", op, scope=scope)
                self.assertEqual(result.returncode, 0, result.stderr)
                state = json.loads(self.state.read_text())
                self.assertFalse(state["enabled"])
                self.assertEqual(state["loaded"], op != "stop")
            self.assertFalse(any("enable" in call or "disable" in call for call in self.calls()))
            loads = [call for call in self.calls() if call[:2] == ["launchctl", "load"]]
            self.assertTrue(loads)
            self.assertTrue(all(call[2] == "-F" for call in loads))

    def test_invalid_service_never_reaches_manager(self):
        for backend in ("systemd", "openrc", "launchd"):
            for service in ("--help", "$(touch bad)", "../svc", "svc\nother"):
                self.assertNotEqual(self.run_script(backend, "start", service=service).returncode, 0)
        self.assertEqual(self.calls(), [])

    def test_shell_syntax(self):
        for script in ROOT.glob("*/*.sh"):
            subprocess.run(["/bin/sh", "-n", str(script)], check=True)

    def test_system_writes_elevate_specific_commands(self):
        for backend in ("systemd", "openrc", "launchd"):
            for op in ("start", "stop", "restart", "enable-autostart", "disable-autostart"):
                with self.subTest(backend=backend, op=op):
                    self.state.write_text(json.dumps({"state": "inactive" if op == "start" else "active",
                                                     "loaded": op != "start", "enabled": True}))
                    self.log.write_text("")
                    result = self.run_script(backend, op, scope="system")
                    self.assertEqual(result.returncode, 0, result.stderr)
                    elevated = [call for call in self.calls() if call[0] == "sudo"]
                    self.assertTrue(elevated, self.calls())
                    self.assertTrue(all(call[1] == "--" for call in elevated))
                    self.assertFalse(any("show" in call or "--exists" in call or "status" in call
                                         or "print" in call for call in elevated))
                    self.assertEqual((result.stdout, result.stderr), ("", ""))

    def test_user_commands_never_elevate(self):
        for backend in ("systemd", "openrc", "launchd"):
            for op in ("start", "stop", "restart", "status", "enable-autostart", "disable-autostart"):
                self.state.write_text(json.dumps({"state": "active", "loaded": True,
                                                 "enabled": True, "sudo_error": True}))
                self.log.write_text("")
                result = self.run_script(backend, op)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    def test_root_does_not_elevate_again(self):
        self.env["FAKE_UID"] = "0"
        for backend in ("systemd", "openrc", "launchd"):
            for op in ("status", "stop", "enable-autostart", "disable-autostart"):
                self.state.write_text(json.dumps({"state": "active", "loaded": True,
                                                 "enabled": True, "sudo_error": True}))
                self.log.write_text("")
                result = self.run_script(backend, op, scope="system")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    def test_status_never_needs_sudo_credentials(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.state.write_text(json.dumps({"state": "active", "loaded": True, "sudo_error": True}))
            self.log.write_text("")
            result = self.run_script(backend, "status", scope="system")
            self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "active\n", ""))
            self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    def test_authentication_failure_never_becomes_missing(self):
        for backend in ("systemd", "openrc", "launchd"):
            self.set_state(sudo_error=True)
            result = self.run_script(backend, "start", scope="system")
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("authentication denied", result.stderr)
            self.assertEqual(result.stdout, "")
            self.assertEqual(sum(call[0] == "sudo" for call in self.calls()), 1)
            if backend != "launchd": self.assertEqual(result.returncode, 23)

    def test_launchd_unloaded_states_need_no_sudo(self):
        for scope, plist in (("system", self.system_plist), ("user", self.plist)):
            for state in ("inactive", "missing"):
                if state == "missing": plist.unlink()
                self.set_state(sudo_error=True)
                result = self.run_script("launchd", "status", scope=scope)
                self.assertEqual((result.returncode, result.stdout, result.stderr), (0, state + "\n", ""))
                self.assertFalse(any(call[0] == "sudo" for call in self.calls()))
                traced = self.run_script("launchd", "status", scope=scope, trace=True)
                self.assertEqual((traced.returncode, traced.stdout), (0, result.stdout), traced.stderr)
                self.assertIn("+", traced.stderr)

    def test_launchd_query_denial_does_not_prompt_or_report_missing(self):
        for scope in ("system", "user"):
            self.set_state(query_error=True, sudo_error=True)
            result = self.run_script("launchd", "status", scope=scope)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertIn("Permission denied", result.stderr)
            self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    def test_launchd_missing_target_with_denied_domain_is_query_failure(self):
        self.set_state(domain_error=True)
        result = self.run_script("launchd", "status", scope="system")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("Cannot query launchd domain", result.stderr)
        self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    def test_launchd_already_active_start_needs_no_sudo(self):
        self.state.write_text(json.dumps({"state": "active", "loaded": True, "sudo_error": True}))
        result = self.run_script("launchd", "start", scope="system")
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, "", ""))
        self.assertFalse(any(call[0] == "sudo" for call in self.calls()))

    @unittest.skipIf(hasattr(os, "geteuid") and os.geteuid() == 0, "root bypasses mode-based read denial")
    def test_launchd_unreadable_plist_paths_fail_closed(self):
        for scope, plist in (("system", self.system_plist), ("user", self.plist)):
            for path in (plist, plist.parent, plist.parent.parent):
                mode = path.stat().st_mode
                try:
                    path.chmod(0)
                    self.set_state(sudo_error=True)
                    result = self.run_script("launchd", "status", scope=scope)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, "")
                    self.assertIn("Cannot", result.stderr)
                    self.assertFalse(any(call[0] == "sudo" for call in self.calls()))
                finally:
                    path.chmod(mode)

    def test_invalid_scope_or_operation_never_reaches_tools(self):
        for backend in ("systemd", "openrc", "launchd"):
            for op, scope in (("status", "admin"), ("invalid", "system")):
                self.assertNotEqual(self.run_script(backend, op, scope=scope).returncode, 0)
        self.assertEqual(self.calls(), [])

    def test_xtrace_is_opt_in_and_preserves_status_stdout(self):
        for backend in ("systemd", "openrc", "launchd"):
            for scope in ("system", "user"):
                for state in ("active", "inactive", "missing"):
                    with self.subTest(backend=backend, scope=scope, state=state):
                        self.state.write_text(json.dumps({"state": state, "loaded": state != "missing",
                                                         "missing": state == "missing"}))
                        if state == "missing":
                            self.plist.unlink(missing_ok=True)
                            self.system_plist.unlink(missing_ok=True)
                        plain = self.run_script(backend, "status", scope=scope)
                        traced = self.run_script(backend, "status", scope=scope, trace=True)
                        self.assertEqual((plain.returncode, plain.stdout, plain.stderr), (0, state + "\n", ""))
                        self.assertEqual((traced.returncode, traced.stdout), (0, plain.stdout), traced.stderr)
                        self.assertIn("+", traced.stderr)


class ResourceTests(unittest.TestCase):
    def test_templates_have_required_actions_and_noninteractive_status(self):
        ids = {"start_core", "stop_core", "restart_core", "status_core", "enable_autostart_core", "disable_autostart_core"}
        for backend in ("systemd", "openrc", "launchd", "windows"):
            name = "system.json" if backend == "windows" else "default.json"
            template = json.loads((ROOT / backend / name).read_text())
            self.assertEqual(template["root"], "scripts/coresrvctrl")
            actions = {action["id"]: action for action in template["actions"]}
            self.assertTrue(ids <= actions.keys())
            self.assertNotIn("label", actions["status_core"])
            self.assertTrue(all(action.get("label") for action_id, action in actions.items() if action_id not in ("status_core", "copy", "edit_file", "open_dir")))
            self.assertTrue(all("cores" not in action for action in actions.values()))
            self.assertFalse(actions["status_core"]["cmd"]["interactive"])
            self.assertEqual(actions["status_core"]["cmd"]["output"], "silent")
            copy = actions["copy"]
            self.assertNotIn("label", copy)
            self.assertFalse(copy["cmd"]["interactive"])
            self.assertEqual(copy["cmd"]["output"], "terminal")
            self.assertTrue(copy["cmd"]["keep_children_on_success"])
            self.assertEqual(copy["cmd"]["timeout_secs"], 5)
            self.assertEqual(actions["status_core"]["cmd"]["timeout_secs"], 5)
            for action_id in ("edit_file", "open_dir"):
                action = actions[action_id]
                self.assertNotIn("label", action)
                command = action["cmd"]
                self.assertFalse(command["interactive"])
                self.assertEqual(command["output"], "silent")
                self.assertTrue(command["keep_children_on_success"])
                self.assertNotIn("timeout_secs", command)
                self.assertEqual(command["args"][-1], "${path}")
                resource = next(a for a in command["args"] if a.startswith("${config_root}/scripts/files/"))
                self.assertTrue((ROOT.parents[1] / resource.removeprefix("${config_root}/")).is_file())
            if backend != "windows":
                for action in (a for a in actions.values() if a["id"] not in ("copy", "edit_file", "open_dir")):
                    self.assertEqual(action["cmd"]["program"], "/bin/sh")
                    args = action["cmd"]["args"]
                    self.assertEqual(args[2], "${scope}")
                    self.assertTrue((ROOT / args[0].removeprefix("${root}/")).is_file())
                    self.assertNotIn("sudo", args)

    def test_windows_bom_and_proxy_disable_has_no_port_dependency(self):
        for script in (ROOT / "windows").glob("*.ps1"):
            self.assertTrue(script.read_bytes().startswith(b"\xef\xbb\xbf"))
        actions = json.loads((ROOT / "windows/system.json").read_text())["actions"]
        disable = next(action for action in actions if action["id"] == "disable_system_proxy")
        self.assertNotIn("${proxy_port}", disable["cmd"]["args"])


if __name__ == "__main__":
    unittest.main()
