#!/usr/bin/env python3
"""Capture real local Cargo fixtures. This is a test helper, NOT a sandbox runner."""
import hashlib,json,os,pathlib,signal,subprocess,sys,time
root=pathlib.Path(__file__).resolve().parent
out=pathlib.Path(sys.argv[1]);out.mkdir(parents=True,exist_ok=True)
version=subprocess.check_output(['cargo','--version'],text=True).strip()
def invoke(argv,timeout=None):
    p=subprocess.Popen(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
    timed_out=False
    try: stdout,stderr=p.communicate(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out=True;os.killpg(p.pid,signal.SIGKILL);stdout,stderr=p.communicate()
    return stdout,stderr,p.returncode,timed_out
for name,selector in [('pass','cases::pass'),('fail','cases::fail'),('skip','cases::ignored'),('zero','no_matching_case'),('partial','cases::pass'),('timeout','cases::timeout'),('missing-report','cases::pass'),('malformed','cases::pass')]:
    args=['cargo','test','--manifest-path',str(root/'native/Cargo.toml'),'--lib',selector,'--']
    inv,inv_err,inv_exit,_=invoke(args+['--list','--format','terse'])
    assert inv_exit==0,inv_err
    command=args+['--format','pretty','--test-threads=1']
    stdout,stderr,exit_code,timed_out=invoke(command,0.4 if name=='timeout' else 20)
    directory=out/name;directory.mkdir(exist_ok=True)
    (directory/'stdout.raw').write_bytes(stdout);(directory/'stderr.raw').write_bytes(stderr);(directory/'inventory.raw').write_bytes(inv)
    # Fault injections preserve original report bytes and record the transform.
    consumed=stdout; fault=None
    if name=='missing-report': consumed=b'';fault='drop collected report after native run'
    if name=='malformed': consumed=stdout[:max(1,len(stdout)//2)];fault='truncate collected report after native run'
    (directory/'report.input').write_bytes(consumed)
    metadata={'tool_version':version,'command':command,'inventory_command':args+['--list','--format','terse'],'exit_code':exit_code,'timed_out':timed_out,'fault_injection':fault,'partial_scope':name=='partial','sha256':{f:hashlib.sha256((directory/f).read_bytes()).hexdigest() for f in ['stdout.raw','stderr.raw','inventory.raw','report.input']}}
    (directory/'capture.json').write_text(json.dumps(metadata,indent=2)+'\n')
print(out)
