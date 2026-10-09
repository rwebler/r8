//! Standalone capacity-prototype checks; uses a port recorder, not a renderer.
use rx82::{bus::Bus, native, system::Device};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Ports { regs: [u8; 11], writes: Vec<(u16, u8)> }
struct Recorder { state: Rc<RefCell<Ports>>, serviced: bool, value: u8 }
impl Device for Recorder {
    fn tick(&mut self, bus: &mut Bus) {
        let next = bus.pending_write.is_some();
        if !bus.mem || !(0xFF30..=0xFF3A).contains(&bus.addr) {
            self.serviced = false;
            return;
        }
        if !self.serviced {
            let mut s = self.state.borrow_mut();
            let i = usize::from(bus.addr - 0xFF30);
            if bus.write {
                s.writes.push((bus.addr, bus.data));
                s.regs[i] = bus.data;
            } else { self.value = s.regs[i]; }
            self.serviced = true;
        }
        if !bus.write { bus.write_data(self.value); }
        if next { self.serviced = false; }
    }
}
fn session(rom: &[u8], source: &str, mode: u8) -> (Vec<(u16,u8)>, String) {
    let (mut sys, console) = native::machine();
    sys.rom_modules[0].data = rom.to_vec();
    sys.rom_modules[0].end = native::ROM_START + u16::try_from(rom.len()).unwrap() - 1;
    let shared = Rc::new(RefCell::new(Ports::default()));
    shared.borrow_mut().regs[0] = mode;
    shared.borrow_mut().regs[5] = 7;
    sys.devices.insert(0, Box::new(Recorder { state: shared.clone(), serviced: false, value: 0 }));
    console.borrow_mut().input.extend(format!("{source}\nQUIT\n").bytes());
    console.borrow_mut().eof = true;
    for _ in 0..5_000_000 {
        if sys.cpu.halt { break; }
        sys.tick();
    }
    assert!(sys.cpu.halt, "timeout: {source}");
    let writes = shared.borrow().writes.clone();
    let output = String::from_utf8(console.borrow().output.clone()).unwrap();
    (writes, output)
}
fn main() {
    let rom = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    assert!(rom.len() <= 0x2E00);
    let (w,o) = session(&rom, "SCREEN 1\nCOLOR 2,7\nPLOT 159,95\nPRINT 42", 0);
    assert_eq!(w, [(0xFF30,1),(0xFF34,2),(0xFF35,7),(0xFF36,159),(0xFF37,95),(0xFF3A,1)]);
    assert!(o.contains("42\n"), "{o}");
    for (mode, count) in [(0,1920), (1,3840)] {
        let (w,o) = session(&rom, "CLS", mode);
        assert!(!o.contains("? "), "{o}");
        assert_eq!(w.len(), count + 2);
        assert_eq!(&w[..2], &[(0xFF31,0),(0xFF32,0)]);
        for (i,&(address,value)) in w[2..].iter().enumerate() {
            assert_eq!(address,0xFF33);
            assert_eq!(value, if mode == 1 {0} else if i % 2 == 0 {0x20} else {0x70});
        }
    }
    for bad in ["SCREEN -1","SCREEN 2","COLOR -1,7","COLOR 0,16","PLOT -1,0","PLOT 160,0","PLOT 0,-1","PLOT 0,96"] {
        let (w,o)=session(&rom,bad,1);
        assert!(w.is_empty(), "{bad}: {w:?}");
        assert!(o.contains("? ILLEGAL QUANTITY"), "{bad}: {o}");
    }
    for bad in ["SCREEN 1,0","CLS 1","COLOR 1","COLOR 1,2,3","PLOT 1","PLOT 1,2,3"] {
        let (w,o)=session(&rom,bad,1);
        assert!(w.is_empty(), "{bad}: {w:?}");
        assert!(o.contains("? "), "{bad}: {o}");
    }
    let (w,o)=session(&rom,"PLOT 0,0\nPRINT 99",0);
    assert!(w.is_empty()); assert!(o.contains("? ILLEGAL QUANTITY")); assert!(o.contains("99\n"));
    let (w,o)=session(&rom,"10 SCREEN 1\n20 FOR I=0 TO 2\n30 IF I<2 THEN GOSUB 100\n40 NEXT I\n50 END\n100 COLOR I+1,7\n110 PLOT I*2,I+1\n120 RETURN\nLIST\nRUN",0);
    assert!(!o.contains("? "), "{o}");
    assert!(o.contains("10 SCREEN 1"), "{o}");
    assert!(o.contains("110 PLOT I*2,I+1"), "{o}");
    assert_eq!(w,[(0xFF30,1),(0xFF34,1),(0xFF35,7),(0xFF36,0),(0xFF37,1),(0xFF3A,1),(0xFF34,2),(0xFF35,7),(0xFF36,2),(0xFF37,2),(0xFF3A,1)]);
    println!("19 native sessions passed: commands, exact clears, ranges, syntax, recovery, serial PRINT, tokenized LIST, IF/FOR/GOSUB, expressions");
}
