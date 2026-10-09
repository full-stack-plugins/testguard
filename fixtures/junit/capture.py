#!/usr/bin/env python3
"""Real Maven/Surefire captures; explicit post-capture fault injections. Not a production runner."""
import hashlib,json,os,pathlib,shutil,signal,subprocess,sys
root=pathlib.Path(__file__).resolve().parent
out=pathlib.Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
mvn=os.environ.get('TESTGUARD_MAVEN','mvn')
common=[mvn,'-B','-o','-f',str(root/'native/pom.xml')]
if os.environ.get('TESTGUARD_MAVEN_REPOSITORY'):common+=['-Dmaven.repo.local='+os.environ['TESTGUARD_MAVEN_REPOSITORY']]
version=subprocess.check_output([mvn,'--version'],text=True)
for name,selector in [('pass','CasesTest#pass'),('fail','CasesTest#fail'),('skip','CasesTest#skip'),('zero','NoSuchTest'),('partial','CasesTest#pass'),('timeout','CasesTest#timeout'),('missing-report','CasesTest#pass'),('malformed','CasesTest#pass')]:
    reportdir=root/'native/target/surefire-reports'
    if reportdir.exists(): shutil.rmtree(reportdir)
    command=common+['-Dtest='+selector,'-Dsurefire.failIfNoSpecifiedTests=false','test']
    p=subprocess.Popen(command,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True);timed_out=False
    try:stdout,stderr=p.communicate(timeout=4 if name=='timeout' else 30)
    except subprocess.TimeoutExpired:
        timed_out=True;os.killpg(p.pid,signal.SIGKILL);stdout,stderr=p.communicate()
    report=reportdir/'TEST-CasesTest.xml';xml=report.read_bytes() if report.exists() else b''
    consumed=xml;fault=None
    if name=='missing-report':consumed=b'';fault='drop report after successful native execution'
    if name=='malformed':consumed=xml[:len(xml)//2];fault='truncate native XML after execution'
    directory=out/name;directory.mkdir(exist_ok=True)
    for file,data in [('stdout.raw',stdout),('stderr.raw',stderr),('report.raw.xml',xml),('report.input.xml',consumed)]: (directory/file).write_bytes(data)
    metadata={'tool_version':version,'surefire_version':'3.5.2','junit_version':'4.13.2','command':command,'exit_code':p.returncode,'timed_out':timed_out,'fault_injection':fault,'partial_scope':name=='partial','sha256':{f:hashlib.sha256((directory/f).read_bytes()).hexdigest() for f in ['stdout.raw','stderr.raw','report.raw.xml','report.input.xml']}}
    (directory/'capture.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(out)
