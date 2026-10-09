#!/usr/bin/env python3
"""Linux offline native fixtures only. Original output and injected consumer faults remain separate.
No candidate runner, credential/network sandbox or general executable interface is provided.
"""
import ctypes, hashlib, json, os, pathlib, re, shutil, signal, subprocess, sys, tempfile, threading, time
import xml.etree.ElementTree as ET
ROOT = pathlib.Path(__file__).resolve().parent
TOOLS = pathlib.Path('/workspace/guard-toolchain')
MAVEN = TOOLS / 'maven/apache-maven-3.9.9/bin/mvn'
GRADLE = TOOLS / 'gradle/gradle-8.14.3/bin/gradle'
JAVA = TOOLS / 'jdk21/usr/lib/jvm/java-21-openjdk-amd64'
REPOSITORY = TOOLS / 'maven/repository'
OUTPUT_LIMIT = 2 * 1024 * 1024
SCENARIOS = {
 'pass': ('CasesTest#pass*', 'CasesTest.pass*', 'CasesTest'),
 'fail': ('CasesTest#fail', 'CasesTest.fail', 'CasesTest'),
 'skip': ('CasesTest#skip', 'CasesTest.skip', 'CasesTest'),
 'zero': ('NoSuchTest', 'NoSuchTest', 'CasesTest'),
 'partial': ('CasesTest#passA', 'CasesTest.passA', 'CasesTest'),
 'timeout': ('CasesTest#timeout', 'CasesTest.timeout', 'CasesTest'),
 'missing-report': ('CasesTest#pass*', 'CasesTest.pass*', 'CasesTest'),
 'malformed': ('CasesTest#pass*', 'CasesTest.pass*', 'CasesTest'),
 'parameter-unique': ('UniqueParametersTest', 'UniqueParametersTest', 'UniqueParametersTest'),
 'parameter-collision': ('CollidingParametersTest', 'CollidingParametersTest', 'CollidingParametersTest'),
}
def digest(data): return hashlib.sha256(data).hexdigest()
def env():
 return {'PATH': '/usr/bin:/bin', 'JAVA_HOME': str(JAVA), 'LANG': 'C.UTF-8', 'LC_ALL': 'C.UTF-8',
         'HOME': os.environ['HOME'], 'GRADLE_USER_HOME': str(TOOLS/'gradle/fixture-home'),
         'TESTGUARD_MAVEN_REPOSITORY': str(REPOSITORY),
         'MAVEN_OPTS': '-Xms64m -Xmx256m -XX:MaxMetaspaceSize=256m -XX:ActiveProcessorCount=2'}
def processes():
 found = {}
 for path in pathlib.Path('/proc').iterdir():
  if not path.name.isdigit(): continue
  try:
   fields = (path/'stat').read_text().rsplit(')',1)[1].split()
   found[int(path.name)] = (int(fields[1]), int(fields[19]))
  except (OSError, ValueError, IndexError): pass
 return found

def descendants():
 all_processes = processes(); owned = {os.getpid()}
 while True:
  new = {pid for pid,(parent,_) in all_processes.items() if parent in owned}
  if new <= owned: break
  owned |= new
 return {pid:all_processes[pid][1] for pid in owned if pid != os.getpid()}

def kill_owned():
 owned = descendants()
 for pid,start in owned.items():
  try:
   fd = os.pidfd_open(pid)
   try:
    if processes().get(pid, (None,None))[1] == start: signal.pidfd_send_signal(fd, signal.SIGKILL)
   finally: os.close(fd)
  except (ProcessLookupError, FileNotFoundError): pass
 return list(owned)

def run(command, scenario, cwd):
 process = subprocess.Popen(command, cwd=cwd, env=env(), stdin=subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
 outputs = [bytearray(), bytearray()]; overflow = threading.Event()
 def read(pipe, dest):
  while True:
   chunk = os.read(pipe.fileno(),4096)
   if not chunk: break
   keep = min(len(chunk), OUTPUT_LIMIT-len(dest)); dest.extend(chunk[:keep])
   if keep != len(chunk): overflow.set()
  pipe.close()
 readers = [threading.Thread(target=read,args=(pipe,dest),daemon=True) for pipe,dest in zip([process.stdout,process.stderr],outputs)]
 for reader in readers: reader.start()
 start=time.monotonic(); ready=None; reason=None; observed=set()
 while process.poll() is None:
  owned=descendants();observed.update(owned)
  if b'TG_TIMEOUT_READY' in outputs[0] or b'TG_TIMEOUT_READY' in outputs[1]: ready=ready or time.monotonic()
  if overflow.is_set(): reason='output_limit'
  elif len(owned)>32: reason='process_limit'
  elif scenario=='timeout' and ready is not None and time.monotonic()-ready>=0.5: reason='testcase_timeout'
  elif time.monotonic()-start>120: reason='startup_or_build_deadline'
  if reason:
   try: os.killpg(process.pid,signal.SIGKILL)
   except ProcessLookupError: pass
   kill_owned();break
  time.sleep(0.02)
 try: code=process.wait(timeout=5)
 except subprocess.TimeoutExpired: kill_owned();code=process.wait(timeout=5)
 cleanup_start=time.monotonic()
 while True:
  observed.update(kill_owned())
  try:
   while True:
    pid,_=os.waitpid(-1,os.WNOHANG)
    if pid==0: break
  except ChildProcessError: pass
  if not descendants(): break
  if time.monotonic()-cleanup_start>5: raise RuntimeError('owned process cleanup budget exceeded')
  time.sleep(0.01)
 for reader in readers: reader.join(timeout=1)
 if any(reader.is_alive() for reader in readers): raise RuntimeError('output EOF cleanup failed')
 if scenario=='timeout' and (reason!='testcase_timeout' or ready is None): raise RuntimeError('timeout fixture did not reach actual testcase: '+str(reason))
 if reason not in (None,'testcase_timeout'): raise RuntimeError('fixture resource failure: '+str(reason))
 return bytes(outputs[0]),bytes(outputs[1]),{'exit_code':code,'termination':reason,'testcase_ready':ready is not None,'wall_seconds':round(time.monotonic()-start,3),'cleanup_complete':True,'owned_pids':sorted(observed),'output_limit_per_pipe':OUTPUT_LIMIT}

def capture(tool, scenario, out):
 destination=out/tool/scenario;destination.mkdir(parents=True,exist_ok=False)
 selectors=SCENARIOS[scenario]
 with tempfile.TemporaryDirectory(prefix='tg-junit-',dir='/workspace/guard-implementation-ledger') as tmp:
  work=pathlib.Path(tmp);shutil.copytree(ROOT/'native',work,dirs_exist_ok=True)
  if tool=='maven':
   command=[str(MAVEN),'-B','-o','-s',str(work/'settings.xml'),'-gs',str(work/'settings.xml'),'-f',str(work/'pom.xml'),'-Dmaven.repo.local='+str(REPOSITORY),'-Dtest='+selectors[0],'-Dsurefire.failIfNoSpecifiedTests=false','test']
   reportdir=work/'target/surefire-reports'
  else:
   command=[str(GRADLE),'--offline','--no-daemon','--max-workers=1','--console=plain','--project-dir',str(work),'test','--tests',selectors[1]]
   reportdir=work/'build/test-results/test'
  assert not reportdir.exists(), 'fresh attempt report root required'
  stdout,stderr,meta=run(command,scenario,work)
  reports=sorted(reportdir.glob('TEST-*.xml')) if reportdir.exists() else []
  expected=reportdir/('TEST-'+selectors[2]+'.xml')
  assert reports==([] if not expected.exists() else [expected]), 'unexpected native suites'
  raw=expected.read_bytes() if expected.exists() else b''
  if len(raw)>4*1024*1024: raise RuntimeError('report size limit')
  expected_exit = 1 if scenario=='fail' else 0
  if scenario!='timeout' and meta['exit_code']!=expected_exit: raise RuntimeError('unexpected native exit; not a successful capture: '+str(meta))
  if scenario not in ['zero','timeout'] and not raw: raise RuntimeError('native report missing before any fault injection')
  consumed=raw;fault=None
  if scenario=='missing-report': consumed=b'';fault='consumer-side removal AFTER real successful native execution; raw XML preserved'
  elif scenario=='malformed':consumed=raw[:len(raw)//2];fault='consumer-side truncation AFTER real successful native execution; raw XML preserved'
  native=None
  if raw:
   root=ET.fromstring(raw);native={'suite':root.attrib['name'],'counts':{k:int(root.attrib[k]) for k in ['tests','failures','errors','skipped']},'cases':[{'class':n.attrib['classname'],'name':n.attrib['name'],'status':next((x.tag for x in n if x.tag in ['failure','error','skipped']),'pass')} for n in root.findall('testcase')]}
  for name,data in [('stdout.raw',stdout),('stderr.raw',stderr),('report.raw.xml',raw),('report.input.xml',consumed)]: (destination/name).write_bytes(data)
  meta.update({'tool':tool,'scenario':scenario,'command':command,'cwd':str(work),'report_path':str(expected),'fresh_report_root':True,'fault_injection':fault,'partial_scope':scenario=='partial','native_xml':native,'profile':{'tool':'maven-surefire' if tool=='maven' else 'gradle','version':'3.5.2' if tool=='maven' else '8.14.3','protocol':'junit-xml-v1'},'junit_version':'4.13.2','required_baseline_methods':['CasesTest.passA','CasesTest.passB'],'sha256':{p.name:digest(p.read_bytes()) for p in destination.iterdir()}})
  text=stdout.decode('utf-8',errors='replace')
  if tool=='maven':
   matches=re.findall(r'Tests run: (\d+), Failures: (\d+), Errors: (\d+), Skipped: (\d+)',text)
   meta['native_console_counts']=dict(zip(['tests','failures','errors','skipped'],map(int,matches[-1]))) if matches else None
  else:
   matches=re.findall(r'TG_NATIVE_COUNTS tests=(\d+) failures=(\d+) skipped=(\d+) successful=(\d+)',text)
   meta['native_console_counts']=dict(zip(['tests','failures','skipped','successful'],map(int,matches[-1]))) if matches else None
  (destination/'capture.json').write_text(json.dumps(meta,indent=2)+'\n')
  print(tool,scenario,'exit',meta['exit_code'],'native',native['counts'] if native else None,'cleanup',meta['cleanup_complete'],flush=True)

if __name__=='__main__':
 if sys.platform!='linux': raise SystemExit('Linux fixture capture only')
 # Only this dedicated capture process adopts/reaps descendants, never a host library.
 if ctypes.CDLL(None,use_errno=True).prctl(36,1,0,0,0)!=0: raise OSError(ctypes.get_errno(),'PR_SET_CHILD_SUBREAPER')
 output=pathlib.Path(sys.argv[1]);selected_tools=sys.argv[2].split(',') if len(sys.argv)>2 else ['maven','gradle'];selected_cases=sys.argv[3].split(',') if len(sys.argv)>3 else list(SCENARIOS)
 assert set(selected_tools)<={'maven','gradle'} and set(selected_cases)<=set(SCENARIOS)
 output.mkdir(parents=True,exist_ok=True)
 provenance={'schema':'testguard.native-capture/v1','sources':{str(p.relative_to(ROOT)):digest(p.read_bytes()) for p in sorted((ROOT/'native').rglob('*')) if p.is_file()},'capture_script_sha256':digest(pathlib.Path(__file__).read_bytes()),'tools':{}}
 for name,path in [('gradle_distribution',TOOLS/'gradle/gradle-8.14.3-bin.zip'),('maven_distribution',TOOLS/'maven/maven.tar.gz'),('java',JAVA/'bin/java'),('javac',JAVA/'bin/javac'),('junit_jar',REPOSITORY/'junit/junit/4.13.2/junit-4.13.2.jar'),('java_modules',JAVA/'lib/modules'),('java_vm',JAVA/'lib/server/libjvm.so'),('surefire_plugin',REPOSITORY/'org/apache/maven/plugins/maven-surefire-plugin/3.5.2/maven-surefire-plugin-3.5.2.jar'),('surefire_junit4_provider',REPOSITORY/'org/apache/maven/surefire/surefire-junit4/3.5.2/surefire-junit4-3.5.2.jar')]:provenance['tools'][name]={'path':str(path),'sha256':digest(path.read_bytes())}
 for name,cmd in [('maven',[str(MAVEN),'--version']),('gradle',[str(GRADLE),'--version']),('java',[str(JAVA/'bin/java'),'-version'])]:
  version=subprocess.run(cmd,env=env(),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=30,check=True).stdout
  assert len(version)<16384;provenance['tools'][name+'_version']=version.decode()
 provenance['gradle_distribution_verification']=json.loads((TOOLS/'gradle/distribution-provenance.json').read_text())
 (output/'provenance.json').write_text(json.dumps(provenance,indent=2)+'\n')
 try:
  for tool in selected_tools:
   for case in selected_cases: capture(tool,case,output)
 finally: kill_owned()
