//! Optional SDL2 display and speaker frontend.
#![allow(unsafe_code, reason = "SDL2 C API")]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "bounded frame/audio counts and elapsed time arithmetic"
)]
use crate::{
    basic::Basic,
    native::ConsoleRoute,
    screen_console::{ScreenConsole, ScreenWriter},
    sound::Sound,
    video::{HEIGHT, Video, WIDTH},
};
use anyhow::{Result, ensure};
use std::{
    cell::RefCell,
    collections::VecDeque,
    ffi::{CStr, c_char, c_int, c_void},
    io::{self, BufReader, Read, Write},
    ptr,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError},
    },
    thread,
    time::{Duration, Instant},
};

const FRAME_TIME: Duration = Duration::from_millis(16);
const AUDIO_LIMIT: u32 = 24_000; // 250 ms at 48 kHz mono, 16 bit.

enum Packet {
    Frame(Box<[u32]>),
    Audio(Vec<i16>),
    Done,
}

#[derive(Clone, Copy)]
enum KeyInput {
    Byte(u8),
    Break,
}

#[repr(C, align(8))]
struct SdlEvent([u8; 64]);

#[repr(C)]
struct AudioSpec {
    freq: c_int,
    format: u16,
    channels: u8,
    silence: u8,
    samples: u16,
    padding: u16,
    size: u32,
    callback: Option<unsafe extern "C" fn(*mut c_void, *mut u8, c_int)>,
    userdata: *mut c_void,
}

#[link(name = "SDL2")]
unsafe extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut c_void;
    fn SDL_DestroyWindow(window: *mut c_void);
    fn SDL_CreateRenderer(window: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
    fn SDL_DestroyRenderer(renderer: *mut c_void);
    fn SDL_RenderSetLogicalSize(renderer: *mut c_void, w: c_int, h: c_int) -> c_int;
    fn SDL_RenderSetIntegerScale(renderer: *mut c_void, enabled: c_int) -> c_int;
    fn SDL_CreateTexture(
        renderer: *mut c_void,
        format: u32,
        access: c_int,
        w: c_int,
        h: c_int,
    ) -> *mut c_void;
    fn SDL_DestroyTexture(texture: *mut c_void);
    fn SDL_UpdateTexture(
        texture: *mut c_void,
        rect: *const c_void,
        pixels: *const c_void,
        pitch: c_int,
    ) -> c_int;
    fn SDL_RenderClear(renderer: *mut c_void) -> c_int;
    fn SDL_RenderCopy(
        renderer: *mut c_void,
        texture: *mut c_void,
        src: *const c_void,
        dst: *const c_void,
    ) -> c_int;
    fn SDL_RenderPresent(renderer: *mut c_void);
    fn SDL_PollEvent(event: *mut c_void) -> c_int;
    fn SDL_StartTextInput();
    fn SDL_StopTextInput();
    fn SDL_OpenAudioDevice(
        device: *const c_char,
        capture: c_int,
        desired: *const AudioSpec,
        obtained: *mut AudioSpec,
        allowed: c_int,
    ) -> u32;
    fn SDL_CloseAudioDevice(device: u32);
    fn SDL_PauseAudioDevice(device: u32, pause: c_int);
    fn SDL_GetQueuedAudioSize(device: u32) -> u32;
    fn SDL_ClearQueuedAudio(device: u32);
    fn SDL_QueueAudio(device: u32, data: *const c_void, len: u32) -> c_int;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdl_text_and_control_events_are_distinct() {
        let mut text = SdlEvent([0_u8; 64]);
        text.0[..4].copy_from_slice(&0x303_u32.to_ne_bytes());
        text.0[12] = b'A';
        let mut key = SdlEvent([0_u8; 64]);
        key.0[..4].copy_from_slice(&0x300_u32.to_ne_bytes());
        key.0[20..24].copy_from_slice(&13_i32.to_ne_bytes());
        assert!(matches!(
            decode_event(&text.0).as_slice(),
            [KeyInput::Byte(b'A')]
        ));
        assert!(matches!(
            decode_event(&key.0).as_slice(),
            [KeyInput::Byte(b'\n')]
        ));
    }
    #[test]
    fn native_frames_skip_emission_but_show_screen_changes() {
        let (mut sys, _) = crate::native::machine_with_console(ConsoleRoute::Screen);
        let (sender, receiver) = mpsc::sync_channel(4);
        sys.cpu.pc = crate::native::SCREEN_EMIT_START;
        emit_native(&sys, &sender, false, ConsoleRoute::Screen);
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        sys.cpu.pc = crate::native::SCREEN_EMIT_END;
        sys.video.borrow_mut().mode = 1;
        sys.video.borrow_mut().picture[0] = 0x40;
        emit_native(&sys, &sender, false, ConsoleRoute::Screen);
        let Packet::Frame(picture) = receiver.try_recv().unwrap() else {
            panic!("expected frame")
        };
        assert_eq!(picture[0], 0xffff_ff00);
        sys.video.borrow_mut().mode = 0;
        emit_native(&sys, &sender, false, ConsoleRoute::Screen);
        let Packet::Frame(text) = receiver.try_recv().unwrap() else {
            panic!("expected frame")
        };
        assert_ne!(text[0], picture[0]);
    }
    #[test]
    fn reference_key_reader_edits_before_delivering_a_line() {
        let basic = Basic::default();
        let screen = Rc::new(RefCell::new(ScreenConsole::new(Rc::clone(&basic.video))));
        let (frame_sender, _) = mpsc::sync_channel(2);
        let stop = Arc::new(AtomicBool::new(false));
        let clock = Rc::new(RefCell::new(LiveClock::new(
            Rc::clone(&basic.video),
            Rc::clone(&basic.sound),
            frame_sender,
            Arc::clone(&stop),
            true,
        )));
        let (key_sender, key_receiver) = mpsc::sync_channel(8);
        for byte in [b'A', b'B', 8, b'C', b'\n'] {
            key_sender.send(KeyInput::Byte(byte)).unwrap();
        }
        let mut reader = KeyRead {
            receiver: key_receiver,
            pending: VecDeque::new(),
            line: Vec::new(),
            overflow: false,
            overflow_signal: Arc::new(AtomicBool::new(false)),
            screen: Rc::clone(&screen),
            clock,
            stop,
        };
        let mut bytes = [0_u8; 8];
        let count = reader.read(&mut bytes).unwrap();
        assert_eq!(&bytes[..count], b"AC\n");
        assert_eq!(&basic.video.borrow().text[..4], &[b'A', 0x67, b'C', 0x67]);
        assert!(!screen.borrow().cursor_visible);
    }
    #[test]
    fn reference_key_reader_rejects_an_overlong_line() {
        let basic = Basic::default();
        let screen = Rc::new(RefCell::new(ScreenConsole::new(Rc::clone(&basic.video))));
        let (frame_sender, _) = mpsc::sync_channel(2);
        let stop = Arc::new(AtomicBool::new(false));
        let clock = Rc::new(RefCell::new(LiveClock::new(
            Rc::clone(&basic.video),
            Rc::clone(&basic.sound),
            frame_sender,
            Arc::clone(&stop),
            true,
        )));
        let (key_sender, key_receiver) = mpsc::sync_channel(2048);
        for _ in 0..1025 {
            key_sender.send(KeyInput::Byte(b'A')).unwrap();
        }
        key_sender.send(KeyInput::Byte(b'\n')).unwrap();
        let mut reader = KeyRead {
            receiver: key_receiver,
            pending: VecDeque::new(),
            line: Vec::new(),
            overflow: false,
            overflow_signal: Arc::new(AtomicBool::new(false)),
            screen,
            clock,
            stop,
        };
        let mut bytes = [0_u8; 8];
        let count = reader.read(&mut bytes).unwrap();
        assert_eq!(&bytes[..count], b"\n");
        let text: String = basic
            .video
            .borrow()
            .text
            .chunks_exact(2)
            .map(|cell| char::from(cell[0]))
            .collect();
        assert!(text.contains("LINE TOO LONG"), "{text}");
    }
    #[test]
    fn reference_key_reader_rejects_a_full_frontend_queue() {
        let basic = Basic::default();
        let screen = Rc::new(RefCell::new(ScreenConsole::new(Rc::clone(&basic.video))));
        let (frame_sender, _) = mpsc::sync_channel(2);
        let stop = Arc::new(AtomicBool::new(false));
        let clock = Rc::new(RefCell::new(LiveClock::new(
            Rc::clone(&basic.video),
            Rc::clone(&basic.sound),
            frame_sender,
            Arc::clone(&stop),
            true,
        )));
        let (key_sender, key_receiver) = mpsc::sync_channel(2);
        key_sender.send(KeyInput::Byte(b'A')).unwrap();
        key_sender.send(KeyInput::Byte(b'B')).unwrap();
        let overflow_signal = Arc::new(AtomicBool::new(true));
        let mut reader = KeyRead {
            receiver: key_receiver,
            pending: VecDeque::new(),
            line: Vec::new(),
            overflow: false,
            overflow_signal,
            screen,
            clock,
            stop,
        };
        let mut bytes = [0_u8; 8];
        let count = reader.read(&mut bytes).unwrap();
        assert_eq!(&bytes[..count], b"\n");
        assert!(matches!(key_sender.try_send(KeyInput::Byte(b'C')), Ok(())));
    }
}

fn decode_event(event: &[u8; 64]) -> Vec<KeyInput> {
    let kind = u32::from_ne_bytes(event[..4].try_into().unwrap_or_default());
    if kind == 0x303 {
        return event[12..44]
            .iter()
            .take_while(|&&byte| byte != 0)
            .filter(|&&byte| byte.is_ascii_graphic() || byte == b' ')
            .copied()
            .map(KeyInput::Byte)
            .collect();
    }
    if kind == 0x300 && event[13] == 0 {
        let sym = i32::from_ne_bytes(event[20..24].try_into().unwrap_or_default());
        let mods = u16::from_ne_bytes(event[24..26].try_into().unwrap_or_default());
        return match sym {
            8 => Some(KeyInput::Byte(8)),
            13 | 1073741912 => Some(KeyInput::Byte(b'\n')),
            99 if mods & 0x00c0 != 0 => Some(KeyInput::Break),
            _ => None,
        }
        .into_iter()
        .collect();
    }
    Vec::new()
}

fn sdl_error() -> String {
    unsafe {
        CStr::from_ptr(SDL_GetError())
            .to_string_lossy()
            .into_owned()
    }
}

struct Backend {
    window: *mut c_void,
    renderer: *mut c_void,
    texture: *mut c_void,
    audio: u32,
    dropped_line: bool,
}
impl Backend {
    fn new() -> Result<Self> {
        unsafe {
            ensure!(
                SDL_Init(0x20) == 0,
                "SDL video initialization: {}",
                sdl_error()
            );
            let window = SDL_CreateWindow(
                c"RX-82".as_ptr(),
                0x2fff0000,
                0x2fff0000,
                960,
                576,
                0x20 | 0x4,
            );
            ensure!(!window.is_null(), "SDL window: {}", sdl_error());
            let mut renderer = SDL_CreateRenderer(window, -1, 2);
            if renderer.is_null() {
                renderer = SDL_CreateRenderer(window, -1, 0);
            }
            ensure!(!renderer.is_null(), "SDL renderer: {}", sdl_error());
            SDL_RenderSetLogicalSize(renderer, WIDTH as c_int, HEIGHT as c_int);
            SDL_RenderSetIntegerScale(renderer, 1);
            let texture =
                SDL_CreateTexture(renderer, 0x1636_2004, 1, WIDTH as c_int, HEIGHT as c_int);
            ensure!(!texture.is_null(), "SDL texture: {}", sdl_error());
            SDL_StartTextInput();
            let desired = AudioSpec {
                freq: 48_000,
                format: 0x8010,
                channels: 1,
                silence: 0,
                samples: 1024,
                padding: 0,
                size: 0,
                callback: None,
                userdata: ptr::null_mut(),
            };
            let audio = if SDL_Init(0x10) == 0 {
                let audio = SDL_OpenAudioDevice(ptr::null(), 0, &desired, ptr::null_mut(), 0);
                if audio == 0 {
                    eprintln!("RX-82 audio unavailable: {}", sdl_error());
                } else {
                    SDL_PauseAudioDevice(audio, 0);
                }
                audio
            } else {
                eprintln!("RX-82 audio unavailable: {}", sdl_error());
                0
            };
            Ok(Self {
                window,
                renderer,
                texture,
                audio,
                dropped_line: false,
            })
        }
    }
    fn present(&mut self, frame: &[u32]) {
        unsafe {
            SDL_UpdateTexture(
                self.texture,
                ptr::null(),
                frame.as_ptr().cast(),
                (WIDTH * 4) as c_int,
            );
            SDL_RenderClear(self.renderer);
            SDL_RenderCopy(self.renderer, self.texture, ptr::null(), ptr::null());
            SDL_RenderPresent(self.renderer);
        }
    }
    fn queue(&mut self, samples: &[i16]) {
        if self.audio == 0 || samples.is_empty() {
            return;
        }
        unsafe {
            if SDL_GetQueuedAudioSize(self.audio) > AUDIO_LIMIT {
                SDL_ClearQueuedAudio(self.audio);
            }
            SDL_QueueAudio(
                self.audio,
                samples.as_ptr().cast(),
                (samples.len() * 2) as u32,
            );
        }
    }
    fn closed(
        &mut self,
        keyboard: Option<&SyncSender<KeyInput>>,
        break_signal: Option<&Arc<AtomicBool>>,
        overflow_signal: Option<&Arc<AtomicBool>>,
    ) -> bool {
        let mut storage = SdlEvent([0_u8; 64]);
        let event = &mut storage.0;
        unsafe {
            while SDL_PollEvent(event.as_mut_ptr().cast()) != 0 {
                let kind = u32::from_ne_bytes(event[..4].try_into().unwrap_or_default());
                if kind == 0x100 || kind == 0x200 && event[12] == 14 {
                    return true;
                }
                let Some(keyboard) = keyboard else { continue };
                for input in decode_event(event) {
                    if matches!(input, KeyInput::Break) {
                        self.dropped_line = false;
                        if let Some(signal) = break_signal {
                            signal.store(true, Ordering::Relaxed);
                        } else {
                            let _ = keyboard.try_send(input);
                        }
                    } else if self.dropped_line {
                        if matches!(input, KeyInput::Byte(b'\n')) {
                            self.dropped_line = false;
                            if let Some(signal) = overflow_signal {
                                signal.store(true, Ordering::Release);
                            }
                        }
                    } else if keyboard.try_send(input).is_err() {
                        if matches!(input, KeyInput::Byte(b'\n')) {
                            if let Some(signal) = overflow_signal {
                                signal.store(true, Ordering::Release);
                            }
                        } else {
                            self.dropped_line = true;
                        }
                    }
                }
            }
        }
        false
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
            SDL_StopTextInput();
            if self.audio != 0 {
                SDL_CloseAudioDevice(self.audio);
            }
            SDL_DestroyTexture(self.texture);
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.window);
            SDL_Quit();
        }
    }
}

fn show(
    receiver: Receiver<Packet>,
    stop: Arc<AtomicBool>,
    keyboard: Option<SyncSender<KeyInput>>,
    break_signal: Option<Arc<AtomicBool>>,
    overflow_signal: Option<Arc<AtomicBool>>,
) -> Result<()> {
    let mut backend = Backend::new()?;
    let mut done = false;
    while !done {
        if backend.closed(
            keyboard.as_ref(),
            break_signal.as_ref(),
            overflow_signal.as_ref(),
        ) {
            stop.store(true, Ordering::Relaxed);
            break;
        }
        match receiver.recv_timeout(FRAME_TIME) {
            Ok(Packet::Frame(mut frame)) => {
                while let Ok(packet) = receiver.try_recv() {
                    match packet {
                        Packet::Frame(newer) => frame = newer,
                        Packet::Audio(samples) => backend.queue(&samples),
                        Packet::Done => done = true,
                    }
                }
                backend.present(&frame);
            }
            Ok(Packet::Audio(samples)) => backend.queue(&samples),
            Ok(Packet::Done) | Err(mpsc::RecvTimeoutError::Disconnected) => done = true,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    stop.store(true, Ordering::Relaxed);
    Ok(())
}

fn emit(
    video: &Rc<RefCell<Video>>,
    sound: &Rc<RefCell<Sound>>,
    sender: &SyncSender<Packet>,
    turbo: bool,
    screen: Option<&Rc<RefCell<ScreenConsole>>>,
) {
    let mut frame = vec![0_u32; WIDTH * HEIGHT].into_boxed_slice();
    video.borrow().render(&mut frame);
    if let Some(screen) = screen {
        let screen = screen.borrow();
        if screen.cursor_visible {
            let cell = screen.cursor_cell().min(959);
            let x = cell % 40 * 8;
            let y = cell / 40 * 8;
            for row in y..y + 8 {
                for pixel in &mut frame[row * WIDTH + x..row * WIDTH + x + 8] {
                    *pixel ^= 0x00ff_ffff;
                }
            }
        }
    }
    for pixel in &mut frame {
        *pixel |= 0xff00_0000;
    }
    let _ = sender.try_send(Packet::Frame(frame));
    let samples: Vec<_> = sound.borrow_mut().drain_samples().collect();
    if !turbo && !samples.is_empty() {
        let _ = sender.try_send(Packet::Audio(samples));
    }
}

fn emit_native(
    sys: &crate::system::System,
    sender: &SyncSender<Packet>,
    turbo: bool,
    route: ConsoleRoute,
) {
    // FF33 uses the selected page. The guest emitter selects text briefly
    // while updating the retained page; present only after it returns.
    let writing_hidden_text = route == ConsoleRoute::Screen
        && (crate::native::SCREEN_EMIT_START..crate::native::SCREEN_EMIT_END).contains(&sys.cpu.pc);
    if !writing_hidden_text {
        let mut frame = vec![0_u32; WIDTH * HEIGHT].into_boxed_slice();
        sys.video.borrow().render(&mut frame);
        if route == ConsoleRoute::Screen && sys.mem.get(0x00F9) == 1 {
            let offset = u16::from(sys.mem.get(0x00F0)) | (u16::from(sys.mem.get(0x00F1)) << 8);
            let cell = usize::from(offset / 2).min(959);
            let x = cell % 40 * 8;
            let y = cell / 40 * 8;
            for row in y..y + 8 {
                for pixel in &mut frame[row * WIDTH + x..row * WIDTH + x + 8] {
                    *pixel ^= 0x00ff_ffff;
                }
            }
        }
        for pixel in &mut frame {
            *pixel |= 0xff00_0000;
        }
        let _ = sender.try_send(Packet::Frame(frame));
    }
    let samples: Vec<_> = sys.sound.borrow_mut().drain_samples().collect();
    if !turbo && !samples.is_empty() {
        let _ = sender.try_send(Packet::Audio(samples));
    }
}

/// Host BASIC device clock and display transport.
pub struct LiveClock {
    video: Rc<RefCell<Video>>,
    sound: Rc<RefCell<Sound>>,
    screen: Option<Rc<RefCell<ScreenConsole>>>,
    break_signal: Option<Arc<AtomicBool>>,
    sender: SyncSender<Packet>,
    stop: Arc<AtomicBool>,
    last: Instant,
    last_frame: Instant,
    tick_fraction: u128,
    turbo: bool,
}
impl LiveClock {
    fn new(
        video: Rc<RefCell<Video>>,
        sound: Rc<RefCell<Sound>>,
        sender: SyncSender<Packet>,
        stop: Arc<AtomicBool>,
        turbo: bool,
    ) -> Self {
        Self {
            video,
            sound,
            screen: None,
            break_signal: None,
            sender,
            stop,
            last: Instant::now(),
            last_frame: Instant::now(),
            tick_fraction: 0,
            turbo,
        }
    }
    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
    pub fn take_break(&self) -> bool {
        self.break_signal
            .as_ref()
            .is_some_and(|signal| signal.swap(false, Ordering::Relaxed))
    }
    pub fn pump(&mut self) {
        let now = Instant::now();
        let hertz = if self.turbo { 16_000_000 } else { 4_000_000 };
        let elapsed = now.duration_since(self.last).as_nanos() * hertz + self.tick_fraction;
        self.tick_fraction = elapsed % 1_000_000_000;
        let ticks = (elapsed / 1_000_000_000).min(u128::from(u64::MAX)) as u64;
        let ticks = if self.turbo { ticks.max(1000) } else { ticks };
        self.last = now;
        if ticks > 0 {
            self.video.borrow_mut().advance(ticks);
            self.sound.borrow_mut().advance(ticks);
        }
        if now.duration_since(self.last_frame) >= FRAME_TIME {
            emit(
                &self.video,
                &self.sound,
                &self.sender,
                self.turbo,
                self.screen.as_ref(),
            );
            self.last_frame = now;
        }
    }
}

struct ChannelRead {
    receiver: Receiver<Option<String>>,
    pending: VecDeque<u8>,
    clock: Rc<RefCell<LiveClock>>,
    stop: Arc<AtomicBool>,
    eof: bool,
}

struct KeyRead {
    receiver: Receiver<KeyInput>,
    pending: VecDeque<u8>,
    line: Vec<u8>,
    overflow: bool,
    overflow_signal: Arc<AtomicBool>,
    screen: Rc<RefCell<ScreenConsole>>,
    clock: Rc<RefCell<LiveClock>>,
    stop: Arc<AtomicBool>,
}
impl Read for KeyRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.screen.borrow_mut().cursor_visible = true;
        while self.pending.is_empty() && !self.stop.load(Ordering::Relaxed) {
            if self.overflow_signal.swap(false, Ordering::AcqRel) {
                while self.receiver.try_recv().is_ok() {}
                self.line.clear();
                self.overflow = false;
                let mut screen = self.screen.borrow_mut();
                for &byte in b"\n? LINE TOO LONG\n" {
                    screen.write_byte(byte);
                }
                self.pending.push_back(b'\n');
                break;
            }
            if self.clock.borrow().take_break() {
                for _ in self.line.drain(..) {
                    self.screen.borrow_mut().backspace();
                }
                self.overflow = false;
                self.screen.borrow_mut().write_byte(b'\n');
                self.pending.push_back(b'\n');
                break;
            }
            match self.receiver.recv_timeout(FRAME_TIME) {
                Ok(KeyInput::Byte(b'\n')) => {
                    self.screen.borrow_mut().write_byte(b'\n');
                    if self.overflow {
                        for &byte in b"? LINE TOO LONG\n" {
                            self.screen.borrow_mut().write_byte(byte);
                        }
                        self.line.clear();
                        self.overflow = false;
                    } else {
                        self.pending.extend(self.line.drain(..));
                    }
                    self.pending.push_back(b'\n');
                }
                Ok(KeyInput::Byte(8 | 127)) => {
                    if self.line.pop().is_some() {
                        self.screen.borrow_mut().backspace();
                    }
                }
                Ok(KeyInput::Byte(byte)) if self.line.len() < 1024 => {
                    self.line.push(byte);
                    self.screen.borrow_mut().write_byte(byte);
                }
                Ok(KeyInput::Byte(_)) => self.overflow = true,
                Ok(KeyInput::Break) => {
                    for _ in self.line.drain(..) {
                        self.screen.borrow_mut().backspace();
                    }
                    self.overflow = false;
                    self.screen.borrow_mut().write_byte(b'\n');
                    self.pending.push_back(b'\n');
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
            self.clock.borrow_mut().pump();
        }
        self.screen.borrow_mut().cursor_visible = false;
        let count = buf.len().min(self.pending.len());
        for byte in &mut buf[..count] {
            *byte = self.pending.pop_front().unwrap_or_default();
        }
        Ok(count)
    }
}
impl Read for ChannelRead {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.pending.is_empty() && !self.eof && !self.stop.load(Ordering::Relaxed) {
            match self.receiver.recv_timeout(FRAME_TIME) {
                Ok(Some(line)) => self.pending.extend(line.bytes()),
                Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => self.eof = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            self.clock.borrow_mut().pump();
        }
        let count = buf.len().min(self.pending.len());
        for byte in &mut buf[..count] {
            *byte = self.pending.pop_front().unwrap_or_default();
        }
        Ok(count)
    }
}

fn stdin_channel() -> Receiver<Option<String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        loop {
            let mut line = String::new();
            match io::stdin().read_line(&mut line) {
                Ok(0) | Err(_) => {
                    let _ = sender.send(None);
                    break;
                }
                Ok(_) => {
                    if sender.send(Some(line)).is_err() {
                        break;
                    }
                }
            }
        }
    });
    receiver
}

/// Runs the reference interpreter with a live SDL window.
pub fn run_reference(
    source: Option<String>,
    turbo: bool,
    random: Option<crate::random::RandomDevice>,
    route: ConsoleRoute,
) -> Result<()> {
    let (sender, receiver) = mpsc::sync_channel(4);
    let (key_sender, key_receiver) = mpsc::sync_channel(2048);
    let break_signal = Arc::new(AtomicBool::new(false));
    let worker_break = Arc::clone(&break_signal);
    let overflow_signal = Arc::new(AtomicBool::new(false));
    let worker_overflow = Arc::clone(&overflow_signal);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || -> Result<()> {
        let mut basic = Basic::default();
        if let Some(random) = random {
            basic.attach_random(random);
        }
        let clock = Rc::new(RefCell::new(LiveClock::new(
            Rc::clone(&basic.video),
            Rc::clone(&basic.sound),
            sender.clone(),
            Arc::clone(&worker_stop),
            turbo,
        )));
        clock.borrow_mut().break_signal = Some(worker_break);
        basic.live_clock = Some(Rc::clone(&clock));
        let result = if route == ConsoleRoute::Screen {
            let screen = Rc::new(RefCell::new(ScreenConsole::new(Rc::clone(&basic.video))));
            clock.borrow_mut().screen = Some(Rc::clone(&screen));
            basic.screen_console = Some(Rc::clone(&screen));
            let mut input = BufReader::new(KeyRead {
                receiver: key_receiver,
                pending: VecDeque::new(),
                line: Vec::new(),
                overflow: false,
                overflow_signal: worker_overflow,
                screen: Rc::clone(&screen),
                clock,
                stop: worker_stop,
            });
            let mut output = ScreenWriter(screen);
            if let Some(source) = source {
                if let Err(error) = basic
                    .load(&source)
                    .and_then(|()| basic.run(&mut input, &mut output))
                {
                    writeln!(output, "? {error:#}")?;
                }
                basic.interact_after_load(&mut input, &mut output)
            } else {
                basic.interact(&mut input, &mut output)
            }
        } else {
            let input = ChannelRead {
                receiver: stdin_channel(),
                pending: VecDeque::new(),
                clock,
                stop: worker_stop,
                eof: false,
            };
            let mut input = BufReader::new(input);
            let mut output = io::stdout().lock();
            if let Some(source) = source {
                basic.load(&source)?;
                basic.run(&mut input, &mut output)
            } else {
                basic.interact(&mut input, &mut output)
            }
        };
        let _ = sender.try_send(Packet::Done);
        result
    });
    let display = show(
        receiver,
        Arc::clone(&stop),
        (route == ConsoleRoute::Screen).then_some(key_sender),
        (route == ConsoleRoute::Screen).then_some(break_signal),
        (route == ConsoleRoute::Screen).then_some(overflow_signal),
    );
    stop.store(true, Ordering::Relaxed);
    let result = worker
        .join()
        .map_err(|_| anyhow::anyhow!("BASIC worker panicked"))?;
    display?;
    result
}

/// Runs native BASIC with a live SDL window and paced device clock.
pub fn run_native(
    source: Option<String>,
    turbo: bool,
    random: Option<crate::random::RandomDevice>,
    route: ConsoleRoute,
) -> Result<()> {
    let (sender, receiver) = mpsc::sync_channel(4);
    let (key_sender, key_receiver) = mpsc::sync_channel(2048);
    let break_signal = Arc::new(AtomicBool::new(false));
    let worker_break = Arc::clone(&break_signal);
    let overflow_signal = Arc::new(AtomicBool::new(false));
    let worker_overflow = Arc::clone(&overflow_signal);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || -> Result<()> {
        let (mut sys, console) = crate::native::machine_with_console(route);
        if let Some(random) = random {
            sys.devices.insert(0, Box::new(random));
        }
        let input = (route == ConsoleRoute::Serial).then(stdin_channel);
        if let Some(source) = &source {
            if route == ConsoleRoute::Screen {
                sys.mem.set(0x00F9, 2); // Preload bypasses interactive echo/editing.
            }
            console
                .borrow_mut()
                .input
                .extend(source.bytes().chain(b"\nRUN\n".iter().copied()));
        }
        let start = Instant::now();
        let mut last_frame = Instant::now();
        let mut ran = false;
        let mut pending_break = false;
        let mut dropping_queued_line = false;
        let mut output = io::stdout().lock();
        while !sys.cpu.halt && !worker_stop.load(Ordering::Relaxed) {
            if route == ConsoleRoute::Screen {
                if worker_break.swap(false, Ordering::AcqRel) {
                    pending_break = true;
                    worker_overflow.store(false, Ordering::Release);
                    dropping_queued_line = false;
                    while key_receiver.try_recv().is_ok() {}
                    let mut state = console.borrow_mut();
                    state.input.clear();
                    state.waiting = false;
                }
                if worker_overflow.swap(false, Ordering::AcqRel) {
                    while key_receiver.try_recv().is_ok() {}
                    dropping_queued_line = false;
                    let accepting = sys.mem.get(0x00F9) == 1 || console.borrow().waiting;
                    if accepting {
                        let mut state = console.borrow_mut();
                        state.input.clear();
                        state.input.extend(std::iter::repeat_n(b'A', 1025));
                        state.input.push_back(b'\n');
                        state.waiting = false;
                    }
                }
                while let Ok(key) = key_receiver.try_recv() {
                    match key {
                        KeyInput::Break => {
                            pending_break = true;
                            dropping_queued_line = false;
                            let mut state = console.borrow_mut();
                            state.input.clear();
                            state.waiting = false;
                        }
                        KeyInput::Byte(byte)
                            if sys.mem.get(0x00F9) == 1 || console.borrow().waiting =>
                        {
                            if dropping_queued_line {
                                if byte == b'\n' {
                                    dropping_queued_line = false;
                                }
                                continue;
                            }
                            let mut state = console.borrow_mut();
                            if state.input.len() < 2048 {
                                state.input.push_back(byte);
                                state.waiting = false;
                            } else {
                                state.input.clear();
                                state.input.extend(std::iter::repeat_n(b'A', 1025));
                                state.input.push_back(b'\n');
                                state.waiting = false;
                                dropping_queued_line = byte != b'\n';
                            }
                        }
                        KeyInput::Byte(_) => {}
                    }
                }
                if pending_break {
                    if sys.cpu.state == crate::state::State::FetchOpcode {
                        sys.cpu.pc = crate::native::SCREEN_BREAK;
                        pending_break = false;
                    } else {
                        sys.tick();
                        continue;
                    }
                }
            }
            let due = (start.elapsed().as_nanos() * 4_000_000 / 1_000_000_000)
                .min(u128::from(u64::MAX)) as u64;
            if !turbo && sys.device_ticks >= due {
                thread::sleep(Duration::from_micros(100));
            } else if console.borrow().waiting {
                if route == ConsoleRoute::Serial
                    && source.is_some()
                    && ran
                    && sys.mem.get(0x0082) == 0
                {
                    let mut state = console.borrow_mut();
                    state.waiting = false;
                    state.input.extend(b"QUIT\n");
                } else {
                    match input
                        .as_ref()
                        .map(Receiver::try_recv)
                        .unwrap_or(Err(TryRecvError::Empty))
                    {
                        Ok(Some(line)) => {
                            let mut state = console.borrow_mut();
                            state.waiting = false;
                            state.input.extend(line.bytes());
                        }
                        Ok(None) | Err(TryRecvError::Disconnected) => {
                            let mut state = console.borrow_mut();
                            state.waiting = false;
                            state.eof = true;
                        }
                        Err(TryRecvError::Empty) => {
                            if !turbo && due > sys.device_ticks {
                                sys.advance_devices((due - sys.device_ticks).min(1000));
                            } else if turbo {
                                sys.advance_devices(16_000);
                                thread::sleep(Duration::from_millis(1));
                            } else {
                                thread::sleep(Duration::from_millis(1));
                            }
                        }
                    }
                }
            } else {
                sys.tick();
                ran |= sys.mem.get(0x0082) != 0;
                if ran && route == ConsoleRoute::Screen && sys.mem.get(0x00F9) == 2 {
                    sys.mem.set(0x00F9, 0);
                }
            }
            let bytes = std::mem::take(&mut console.borrow_mut().output);
            if !bytes.is_empty() {
                output.write_all(&bytes)?;
                output.flush()?;
            }
            if last_frame.elapsed() >= FRAME_TIME {
                emit_native(&sys, &sender, turbo, route);
                last_frame = Instant::now();
            }
        }
        let _ = sender.try_send(Packet::Done);
        Ok(())
    });
    let display = show(
        receiver,
        Arc::clone(&stop),
        (route == ConsoleRoute::Screen).then_some(key_sender),
        (route == ConsoleRoute::Screen).then_some(break_signal),
        (route == ConsoleRoute::Screen).then_some(overflow_signal),
    );
    stop.store(true, Ordering::Relaxed);
    let result = worker
        .join()
        .map_err(|_| anyhow::anyhow!("native worker panicked"))?;
    display?;
    result
}
