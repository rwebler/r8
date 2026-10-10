use rx82::{basic::Basic, native, system::System};

fn host(source: &str) -> (Basic, Vec<u8>) {
    let mut basic = Basic::default();
    let mut output = Vec::new();
    basic.interact(&mut source.as_bytes(), &mut output).unwrap();
    (basic, output)
}

fn guest(source: &str) -> (System, Vec<u8>) {
    let (mut sys, console) = native::machine();
    console.borrow_mut().input.extend(source.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..15_000_000 {
        sys.tick();
        if sys.cpu.halt {
            break;
        }
    }
    assert!(sys.cpu.halt, "guest did not terminate");
    let output = console.borrow().output.clone();
    (sys, output)
}

#[test]
fn both_interpreters_share_video_statement_results() {
    let source = "COLOR 2,4\nCLS\nSCREEN 1\nPLOT 0,0\nPLOT 3,0\nPRINT \"serial\"\nSCREEN 0\nPRINT \"again\"\nQUIT\n";
    let (host, host_output) = host(source);
    let (guest, guest_output) = guest(source);
    let hv = host.video.borrow();
    let gv = guest.video.borrow();
    assert_eq!(hv.mode, gv.mode);
    assert_eq!(hv.ink, gv.ink);
    assert_eq!(hv.paper, gv.paper);
    assert_eq!(hv.picture, gv.picture);
    assert_eq!(hv.text, gv.text);
    assert_eq!(hv.picture[0], 0x41);
    assert_eq!(&hv.text[..4], &[0x20, 0x42, 0x20, 0x42]);
    assert!(host_output.windows(6).any(|w| w == b"serial"));
    assert!(guest_output.windows(6).any(|w| w == b"serial"));
}

#[test]
fn invalid_statements_leave_video_unchanged() {
    let source = "SCREEN 2\nCOLOR 1,16\nSCREEN 1\nPLOT 160,0\nQUIT\n";
    let (host, _) = host(source);
    let (guest, _) = guest(source);
    assert_eq!(host.video.borrow().picture, [0; 3840]);
    assert_eq!(guest.video.borrow().picture, [0; 3840]);
    assert_eq!(host.video.borrow().ink, 7);
    assert_eq!(guest.video.borrow().ink, 7);
}

#[test]
fn debugger_reads_service_ports_without_advancing_time() {
    let mut sys = System {
        turbo: true,
        ..System::default()
    };
    sys.video.borrow_mut().write(0xff31, 7);
    let before = sys.device_ticks;
    assert_eq!(sys.peek_mem(0xff31), 7);
    assert_eq!(sys.peek_mem(0xff30), 0);
    sys.video.borrow_mut().write(0xff31, 0);
    assert_eq!(sys.peek_mem(0xff33), 0x20);
    assert_eq!(sys.peek_mem(0xff33), 0x67);
    assert_eq!(sys.device_ticks, before);
    sys.advance_devices(60_000);
    assert_eq!(sys.peek_mem(0xff30), 0x80);
    assert_eq!(sys.device_ticks, 60_000);
    sys.cpu.reset(&mut sys.bus);
    assert_eq!(sys.video.borrow().phase, 3_600_000);
}

#[test]
fn tokenized_programs_list_and_execute_new_commands() {
    let source = "10 SCREEN 1\n20 COLOR 2,4\n30 PLOT 159,95\n40 SCREEN 0\nRUN\nLIST\nQUIT\n";
    let (host, host_output) = host(source);
    let (guest, guest_output) = guest(source);
    assert_eq!(host.video.borrow().picture[3839], 1);
    assert_eq!(guest.video.borrow().picture[3839], 1);
    for output in [host_output, guest_output] {
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("SCREEN 1"));
        assert!(output.contains("COLOR 2,4"));
        assert!(output.contains("PLOT 159,95"));
    }
}

#[test]
fn raw_video_and_sound_ports_match_between_interpreters() {
    let source = "SCREEN 1\nPOKE 65331,27\nPOKE 65329,0\nPOKE 65330,0\nPRINT PEEK(65331)\nPOKE 65345,255\nPOKE 65350,255\nPOKE 65351,147\nPRINT PEEK(65345),PEEK(65350),PEEK(65351)\nQUIT\n";
    let (host, host_output) = host(source);
    let (guest, guest_output) = guest(source);
    assert_eq!(host.video.borrow().picture, guest.video.borrow().picture);
    assert_eq!(
        host.sound.borrow().registers,
        guest.sound.borrow().registers
    );
    assert_eq!(host.sound.borrow().registers[1], 15);
    assert!(host_output.windows(2).any(|bytes| bytes == b"27"));
    assert!(guest_output.windows(2).any(|bytes| bytes == b"27"));
}
