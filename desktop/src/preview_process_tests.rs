use super::*;
fn fixture() -> Plan {
    Plan {
        width: 160,
        height: 90,
        fps: 30,
        duration: 10.,
        audio: None,
        files: vec![],
        layers: vec![],
    }
}
fn fake(delay: bool) -> PreviewProcess {
    let mut command = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" });
    command.arg("-u").arg("-c").arg(r#"
import json,struct,sys,time,os
r=sys.stdin.buffer; w=sys.stdout.buffer

def read():
    n=r.read(4)
    if not n: raise EOFError()
    payload=json.loads(r.read(struct.unpack('<I',n)[0])); n=struct.unpack('<I',r.read(4))[0];r.read(n)
    return payload

def send(event):
    data=json.dumps(event).encode();w.write(struct.pack('<I',len(data))+data+struct.pack('<I',0));w.flush()

read()
if len(sys.argv)>1: time.sleep(60)
s={'frame':0,'playing':False,'total':300,'rate':1,'phase':'paused','error':None}
send({'Ready':{'Ok':s}})
while True:
    request=read()['Control'];c=request['command'];frame=c.get('frame',0)
    if frame==999: os._exit(11)
    if frame==777: time.sleep(60)
    if frame==123: time.sleep(.3)
    s['frame']=frame
    send({'Reply':{'id':request['id'],'result':{'Ok':s}}})
    if c['action']=='close': break
"#);
    if delay {
        command.arg("delay");
    }
    PreviewProcess::spawn(command, fixture()).unwrap()
}
#[test]
fn snapshot_reads_do_not_wait_for_control_and_crash_fails_pending_call() {
    let p = Arc::new(fake(false));
    p.ready().unwrap();
    let copy = p.clone();
    let task = std::thread::spawn(move || copy.control(Command::Seek { frame: 123 }));
    std::thread::sleep(Duration::from_millis(30));
    let start = Instant::now();
    assert_eq!(p.snapshot().unwrap().total, 300);
    assert!(p.frame(0).is_empty());
    assert!(start.elapsed() < Duration::from_millis(100));
    assert_eq!(task.join().unwrap().unwrap().frame, 123);
    assert!(p.control(Command::Seek { frame: 999 }).is_err());
    assert!(p.snapshot().is_err());
    // A fresh worker remains available after a crash.
    let next = fake(false);
    next.ready().unwrap();
    assert!(next.control(Command::Play).is_ok());
}
#[test]
fn shutdown_cancels_a_hung_operation_and_reaps_worker() {
    let p = Arc::new(fake(false));
    p.ready().unwrap();
    let copy = p.clone();
    let task = std::thread::spawn(move || copy.control(Command::Seek { frame: 777 }));
    std::thread::sleep(Duration::from_millis(30));
    let start = Instant::now();
    p.shutdown();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(task.join().unwrap().is_err());
    assert!(p.child.lock().unwrap().try_wait().unwrap().is_some());
}
#[test]
fn close_interrupts_worker_initialization() {
    let p = Arc::new(fake(true));
    let copy = p.clone();
    let task = std::thread::spawn(move || copy.ready());
    p.shutdown();
    assert!(task.join().unwrap().is_err());
}
#[test]
fn ipc_limits_and_command_validation_reject_invalid_input() {
    assert!(Command::Rate { rate: 0.3 }.engine_args().is_err());
    let bytes = u32::MAX.to_le_bytes();
    assert!(ipc::read::<Event>(&mut bytes.as_slice()).is_err());
    assert!(serde_json::from_str::<Command>(r#"{"action":"rate","frame":8}"#).is_err());
}
