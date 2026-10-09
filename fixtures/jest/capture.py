#!/usr/bin/env python3
"""Real Jest fixture reports, native FILE discovery, separate controller-authored required cases."""
import ctypes,hashlib,json,os,pathlib,selectors,shutil,signal,subprocess,sys,tempfile,time
ROOT=pathlib.Path(__file__).resolve().parent
PACKAGE=pathlib.Path('/workspace/guard-toolchain/jest-30.2.0/node_modules/jest')
NODE=pathlib.Path('/opt/codex/runtimes/codex-primary-runtime/dependencies/node/bin/node')
assert subprocess.check_output([str(NODE),'--version']).decode().strip()=='v24.19.0'
assert json.loads((PACKAGE/'package.json').read_text())['version']=='30.2.0'
CLI_VERSION=subprocess.check_output([str(NODE),str(PACKAGE/'bin/jest.js'),'--version']).decode().strip()
assert CLI_VERSION=='30.1.3'
assert ctypes.CDLL(None).prctl(36,1,0,0,0)==0
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
 with tempfile.TemporaryDirectory(prefix='testguard-jest-',dir='/workspace') as td:
  w=pathlib.Path(td)
  for f in (ROOT/'native').iterdir():
   if f.is_file():shutil.copyfile(f,w/f.name)
  # This manifest is fixture-controller authored BEFORE either native command.
  # Jest --listTests supplies only files and is never called case discovery.
  names={'pass':['first case'],'fail':['failing assertion'],'skip':['explicit skip'],'zero':['required-but-not-discovered'],'partial':['first case','second case'],'timeout':['first case','hanging case'],'missing-report':['first case'],'malformed':['first case'],'hard-timeout':['first case','hanging case'],'collision':['duplicate parameter title','duplicate parameter title']}[name]
  manifest={'schema_version':'testguard.jest-fixture/v1','distribution_version':'30.2.0','cli_version':CLI_VERSION,'core_digest':sha((PACKAGE.parent/'@jest/core/build/index.js').read_bytes()),'target':'cases.test.cjs','required_names':names,'source_digest':sha((w/'cases.test.cjs').read_bytes()),'config_digest':sha((w/'jest.config.cjs').read_bytes())}
  (d/'controller-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
  env={'PATH':str(NODE.parent)+':/usr/bin:/bin','HOME':str(w),'TMPDIR':str(w),'TZ':'UTC','CI':'1','NO_COLOR':'1','TESTGUARD_SCENARIO':name,'TESTGUARD_STARTED':str(w/'started')}
  base=[str(NODE),str(PACKAGE/'bin/jest.js'),'--config',str(w/'jest.config.cjs'),'--runInBand','--no-cache']
  discovery=base+['--listTests','--json']
  imeta,inventory,istderr=execute(discovery,w,env)
  report=w/'report.json'
  if name=='missing-report':
   (w/'blocked').write_text('regular file blocks reporter directory');report=w/'blocked'/'report.json'
  command=base+['--json','--outputFile',str(report),'--passWithNoTests']+(['--testNamePattern','first case'] if name=='partial' else [])
  meta,stdout,stderr=execute(command,w,env,w/'started' if name=='hard-timeout' else None)
  raw=report.read_bytes() if report.is_file() else b''
  consumed=raw;fault=None
  if name=='malformed':
   with report.open('rb') as reader:consumed=reader.read(128);overflow=reader.read(1)
   assert overflow;fault='actual bounded consumer read exhausted at128 bytes; native reporter JSON itself remains intact'
  for file,data in [('inventory.json',inventory),('inventory.stderr',istderr),('report.native.json',raw),('report.input.json',consumed),('stdout.raw',stdout),('stderr.raw',stderr)]: (d/file).write_bytes(data)
  meta.update({'framework':'jest','distribution_version':'30.2.0','cli_version':CLI_VERSION,'protocol':'jest-json-distribution-v30.2.0','required_cases_origin':'explicit fixture-controller manifest, NOT native listTests','native_command':command,'native_cwd':str(w),'native_environment':env,'discovery_command':discovery,'discovery':imeta,'collector_fault':fault,'sha256':{f:sha((d/f).read_bytes()) for f in ['controller-manifest.json','inventory.json','report.native.json','report.input.json','stdout.raw','stderr.raw']}})
  (d/'capture.json').write_text(json.dumps(meta,indent=2)+'\n')
  print(name,meta['exit_code'],len(raw),meta['reason'],flush=True)
manifest={'profile':'local fixed fixture; no OS sandbox or production authority','node':subprocess.check_output([str(NODE),'--version']).decode().strip(),'jest_distribution':'30.2.0','actual_cli_version':CLI_VERSION,'tool_sha256':{str(p):sha(p.read_bytes()) for p in [NODE,PACKAGE/'package.json',PACKAGE/'bin/jest.js',PACKAGE.parent/'@jest/test-result/build/index.js',PACKAGE.parent/'@jest/core/package.json',PACKAGE.parent/'@jest/core/build/index.js',PACKAGE.parent.parent/'package-lock.json']},'source_sha256':{p.name:sha(p.read_bytes()) for p in (ROOT/'native').iterdir() if p.is_file()},'capture_script_sha256':sha(pathlib.Path(__file__).read_bytes())}
(OUT/'PROVENANCE.json').write_text(json.dumps(manifest,indent=2)+'\n')
