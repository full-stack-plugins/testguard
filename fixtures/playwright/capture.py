#!/usr/bin/env python3
"""Real fixed Playwright/system Chromium fixtures. No production runner or sandbox claim."""
import ctypes,hashlib,json,os,pathlib,selectors,shutil,signal,subprocess,sys,tempfile,time
ROOT=pathlib.Path(__file__).resolve().parent
PACKAGE=pathlib.Path('/opt/codex/runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')
NODE=pathlib.Path('/opt/codex/runtimes/codex-primary-runtime/dependencies/node/bin/node')
assert json.loads((PACKAGE/'package.json').read_text())['version']=='1.62.1'
assert subprocess.check_output(['/usr/bin/chromium','--version']).decode().startswith('Chromium 151.0.7922.173 ')
assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0 # Child-only fixture supervisor adopts orphaned descendants.
OUT=pathlib.Path(sys.argv[1]).resolve();OUT.mkdir(parents=True,exist_ok=True)
sha=lambda b:hashlib.sha256(b).hexdigest()
def execute(command,work,env,marker=None):
 p=subprocess.Popen(command,cwd=work,env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
 poll=selectors.DefaultSelector();poll.register(p.stdout,selectors.EVENT_READ,'stdout');poll.register(p.stderr,selectors.EVENT_READ,'stderr')
 buffers={'stdout':bytearray(),'stderr':bytearray()};deadline=time.monotonic()+30;reason=None;started=False
 while poll.get_map():
  if marker and marker.exists():started=True;reason='hard-timeout-after-real-body-start'
  if time.monotonic()>deadline:reason=reason or 'outer-deadline'
  if reason:
   try:os.killpg(p.pid,signal.SIGKILL)
   except ProcessLookupError:pass
  for key,_ in poll.select(.02):
   chunk=os.read(key.fileobj.fileno(),65536)
   if not chunk:poll.unregister(key.fileobj);continue
   target=buffers[key.data];left=4*1024*1024-len(target);target.extend(chunk[:max(0,left)])
   if len(chunk)>left:reason='output-budget'
 p.wait(timeout=5)
 # Killing the complete group includes browser/worker descendants; reap adopted children.
 end=time.monotonic()+10;adopted=set();reaped=0
 while time.monotonic()<end:
  # Playwright starts browsers in their own sessions. A group kill alone cannot
  # reach them; after parent death the child-only subreaper owns those orphans.
  children=[]
  for child_path in pathlib.Path('/proc').iterdir():
   if not child_path.name.isdigit():continue
   try:ppid=int((child_path/'stat').read_text().rsplit(')',1)[1].split()[1])
   except (OSError,ValueError,IndexError):continue
   if ppid==os.getpid():children.append(child_path.name)
  for child_id in children:
   adopted.add(int(child_id))
   try:os.kill(int(child_id),signal.SIGKILL)
   except ProcessLookupError:pass
  done=False
  while True:
   try:child,_=os.waitpid(-1,os.WNOHANG)
   except ChildProcessError:done=True;break
   if child==0:break
   reaped+=1
  if done:break
  time.sleep(.01)
 else:raise RuntimeError('unreaped fixture child')
 return {'exit_code':p.returncode,'interrupted':reason is not None,'reason':reason,'body_started':started,'adopted_children_reaped':reaped},bytes(buffers['stdout']),bytes(buffers['stderr'])
for name in ['pass','fail','skip','zero','partial','timeout','missing-report','malformed','hard-timeout','collision']:
 d=OUT/name;d.mkdir(exist_ok=True)
 with tempfile.TemporaryDirectory(prefix='testguard-playwright-',dir='/workspace') as td:
  w=pathlib.Path(td)
  for f in (ROOT/'native').iterdir():shutil.copyfile(f,w/f.name)
  env={'PATH':str(NODE.parent)+':/usr/bin:/bin','HOME':str(w),'TMPDIR':str(w),'TZ':'UTC','CI':'1','FORCE_COLOR':'0','TESTGUARD_PLAYWRIGHT_PACKAGE':str(PACKAGE),'TESTGUARD_SCENARIO':name,'TESTGUARD_STARTED':str(w/'started'),'TESTGUARD_REPORT':str(w/'inventory.json')}
  base=[str(NODE),str(PACKAGE/'cli.js'),'test','--config',str(w/'playwright.config.cjs'),'--pass-with-no-tests']
  imeta,istdout,istderr=execute(base+['--list'],w,env)
  inventory=(w/'inventory.json').read_bytes() if (w/'inventory.json').exists() else b''
  env['TESTGUARD_REPORT']=str(w/'report.json');command=base+(['--grep','first case'] if name=='partial' else [])
  if name=='missing-report':
   (w/'blocked').write_text('regular file blocks reporter directory');env['TESTGUARD_REPORT']=str(w/'blocked'/'report.json')
  meta,stdout,stderr=execute(command,w,env,w/'started' if name=='hard-timeout' else None)
  report=pathlib.Path(env['TESTGUARD_REPORT']);raw=report.read_bytes() if report.is_file() else b''
  consumed=raw;fault=None
  if name=='malformed':
   with report.open('rb') as reader:consumed=reader.read(128);overflow=reader.read(1)
   assert overflow;fault='actual bounded consumer read exhausted at128 bytes; native reporter JSON itself remains intact'
  for file,data in [('inventory.json',inventory),('inventory.stdout',istdout),('inventory.stderr',istderr),('report.native.json',raw),('report.input.json',consumed),('stdout.raw',stdout),('stderr.raw',stderr)]: (d/file).write_bytes(data)
  meta.update({'framework':'playwright','version':'1.62.1','protocol':'playwright-json-v1.62.1','browser':'system Chromium151.0.7922.173','project':'system-chromium','native_command':command,'native_cwd':str(w),'native_environment':env,'discovery_command':base+['--list'],'discovery':imeta,'collector_fault':fault,'sha256':{f:sha((d/f).read_bytes()) for f in ['inventory.json','report.native.json','report.input.json','stdout.raw','stderr.raw']}})
  (d/'capture.json').write_text(json.dumps(meta,indent=2)+'\n')
  print(name,meta['exit_code'],len(raw),meta['reason'],flush=True)
manifest={'profile':'local fixed fixture; no OS sandbox or production authority','node':subprocess.check_output([str(NODE),'--version']).decode().strip(),'playwright':'1.62.1','chromium':subprocess.check_output(['/usr/bin/chromium','--version']).decode().strip(),'tool_sha256':{str(p):sha(p.read_bytes()) for p in [NODE,PACKAGE/'package.json',PACKAGE/'cli.js',PACKAGE/'lib/program.js',pathlib.Path('/usr/lib/chromium/chromium')]},'source_sha256':{p.name:sha(p.read_bytes()) for p in (ROOT/'native').iterdir()},'capture_script_sha256':sha(pathlib.Path(__file__).read_bytes())}
(OUT/'PROVENANCE.json').write_text(json.dumps(manifest,indent=2)+'\n')
