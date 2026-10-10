use rx82::{native, system::System};
fn session(rom: &[u8], source: &str, serial: bool) -> (System, String) {
    let (mut sys, console) = native::machine();
    sys.rom_modules[0].data = rom.to_vec();
    sys.rom_modules[0].end = native::ROM_START + u16::try_from(rom.len()).unwrap() - 1;
    sys.mem.set(0x00f3, u8::from(serial));
    console.borrow_mut().input.extend(source.bytes());
    console.borrow_mut().eof = true;
    for _ in 0..30_000_000 {
        if sys.cpu.halt { break; }
        sys.tick();
    }
    assert!(sys.cpu.halt, "timeout: {source}");
    let output = String::from_utf8(console.borrow().output.clone()).unwrap();
    (sys, output)
}
fn text(sys: &System) -> String {
    sys.video.borrow().text.chunks_exact(2).map(|c| char::from(c[0])).collect()
}
fn main() {
    let rom = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let (s,o) = session(&rom, "PRINT 42\nQUIT\n", false);
    assert!(o.is_empty());
    assert!(text(&s).contains("RX-82 NATIVE BASIC"));
    assert!(text(&s).contains("42"));
    let (s,o) = session(&rom, "PRINT 42\nQUIT\n", true);
    assert!(o.contains("42\n"));
    assert!(s.video.borrow().text.chunks_exact(2).all(|c| c[0] == b' '));
    let (s,_) = session(&rom, "10 SCREEN 1\n20 PLOT 0,0\n30 PRINT 12345\n40 END\nRUN\nQUIT\n", false);
    assert_eq!(s.video.borrow().mode, 0);
    assert_ne!(s.video.borrow().picture[0], 0);
    assert!(text(&s).contains("12345"));
    let (s,_) = session(&rom, "COLOR 2,4\nCLS\nPRINT \"COLOR\"\nQUIT\n", false);
    let v = s.video.borrow();
    assert!(v.text.chunks_exact(2).all(|c| c[1] == 0x42));
    drop(v);
    let (s,_) = session(&rom, "5 CLS\n10 FOR I=1 TO 50\n20 PRINT I\n30 NEXT I\nRUN\nQUIT\n", false);
    assert!(text(&s).contains("50"));
    assert!(!text(&s).contains("RX-82 NATIVE BASIC"));
    assert!(text(&s).contains("50"));
    let long = "X".repeat(90);
    let (s,_) = session(&rom, &format!("CLS\nPRINT \"{long}\"\nQUIT\n"), false);
    assert!(text(&s).contains(&long));
    let (s,_) = session(&rom, "10 SCREEN 1\n20 PRINT 1/0\nRUN\nQUIT\n", false);
    assert_eq!(s.video.borrow().mode,0);
    assert!(text(&s).contains("DIVISION BY ZERO"));
    let (s,_) = session(&rom, "POKE 65329,123\nPOKE 65330,2\nPRINT 987\nQUIT\n", false);
    // Returning to the direct prompt intentionally resets the pointer via SCREEN 0.
    assert_eq!(s.video.borrow().pointer, 0);
    assert!(text(&s).contains("987"));
    let (s,_) = session(&rom, "10 SCREEN 1\n20 PLOT 0,0\n30 POKE 65329,123\n40 POKE 65330,2\n50 PRINT 12345\n60 QUIT\nRUN\n", false);
    assert_eq!(s.video.borrow().mode, 1);
    assert_eq!(s.video.borrow().pointer, 0x027b);
    assert_eq!(s.video.borrow().picture[0], 0x40);
    assert!(s.video.borrow().picture[1..].iter().all(|&b| b == 0));
    assert!(text(&s).contains("12345"));
    let (s,_) = session(&rom, "10 INPUT A\n20 PRINT A+1\nRUN\n41\nLIST\nQUIT\n", false);
    assert!(text(&s).contains("42"));
    assert!(text(&s).contains("10 INPUT A"));
    let path = std::env::temp_dir().join(format!("rx82-screen-save-{}.bas", std::process::id()));
    assert!(!path.exists());
    let (_,o) = session(&rom, &format!("10 PRINT 123\nSAVE \"{}\"\nQUIT\n", path.display()), false);
    assert!(o.is_empty());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "10 PRINT 123\n");
    std::fs::remove_file(path).unwrap();
    println!("11 native sessions passed: boot/PRINT, serial, picture retention, colors/CLS, scrolling, wrapping, error return, pointer reset");
}
