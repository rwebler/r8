//! Optional SDL2 display and speaker frontend.
#![allow(unsafe_code, reason = "SDL2 C API")]
#![allow(
    clippy::arithmetic_side_effects,
    reason = "bounded frame/audio counts and elapsed time arithmetic"
)]
use crate::{
    basic::Basic,
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
    fn closed(&self) -> bool {
        let mut event = [0_u8; 64];
        unsafe {
            while SDL_PollEvent(event.as_mut_ptr().cast()) != 0 {
                let kind = u32::from_ne_bytes(event[..4].try_into().unwrap_or_default());
                if kind == 0x100 || kind == 0x200 && event[12] == 14 {
                    return true;
                }
            }
        }
        false
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        unsafe {
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

fn show(receiver: Receiver<Packet>, stop: Arc<AtomicBool>) -> Result<()> {
    let mut backend = Backend::new()?;
    let mut done = false;
    while !done {
        if backend.closed() {
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
) {
    let mut frame = vec![0_u32; WIDTH * HEIGHT].into_boxed_slice();
    video.borrow().render(&mut frame);
    for pixel in &mut frame {
        *pixel |= 0xff00_0000;
    }
    let _ = sender.try_send(Packet::Frame(frame));
    let samples: Vec<_> = sound.borrow_mut().drain_samples().collect();
    if !turbo && !samples.is_empty() {
        let _ = sender.try_send(Packet::Audio(samples));
    }
}

/// Host BASIC device clock and display transport.
pub struct LiveClock {
    video: Rc<RefCell<Video>>,
    sound: Rc<RefCell<Sound>>,
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
            emit(&self.video, &self.sound, &self.sender, self.turbo);
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
) -> Result<()> {
    let (sender, receiver) = mpsc::sync_channel(4);
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
        basic.live_clock = Some(Rc::clone(&clock));
        let input = ChannelRead {
            receiver: stdin_channel(),
            pending: VecDeque::new(),
            clock,
            stop: worker_stop,
            eof: false,
        };
        let mut input = BufReader::new(input);
        let mut output = io::stdout().lock();
        let result = if let Some(source) = source {
            basic.load(&source)?;
            basic.run(&mut input, &mut output)
        } else {
            basic.interact(&mut input, &mut output)
        };
        let _ = sender.try_send(Packet::Done);
        result
    });
    let display = show(receiver, Arc::clone(&stop));
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
) -> Result<()> {
    let (sender, receiver) = mpsc::sync_channel(4);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::spawn(move || -> Result<()> {
        let (mut sys, console) = crate::native::machine();
        if let Some(random) = random {
            sys.devices.insert(0, Box::new(random));
        }
        let input = stdin_channel();
        if let Some(source) = &source {
            console
                .borrow_mut()
                .input
                .extend(source.bytes().chain(b"\nRUN\n".iter().copied()));
        }
        let start = Instant::now();
        let mut last_frame = Instant::now();
        let mut ran = false;
        let mut output = io::stdout().lock();
        while !sys.cpu.halt && !worker_stop.load(Ordering::Relaxed) {
            let due = (start.elapsed().as_nanos() * 4_000_000 / 1_000_000_000)
                .min(u128::from(u64::MAX)) as u64;
            if !turbo && sys.device_ticks >= due {
                thread::sleep(Duration::from_micros(100));
            } else if console.borrow().waiting {
                if source.is_some() && ran && sys.mem.get(0x0082) == 0 {
                    let mut state = console.borrow_mut();
                    state.waiting = false;
                    state.input.extend(b"QUIT\n");
                } else {
                    match input.try_recv() {
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
            }
            let bytes = std::mem::take(&mut console.borrow_mut().output);
            if !bytes.is_empty() {
                output.write_all(&bytes)?;
                output.flush()?;
            }
            if last_frame.elapsed() >= FRAME_TIME {
                emit(&sys.video, &sys.sound, &sender, turbo);
                last_frame = Instant::now();
            }
        }
        let _ = sender.try_send(Packet::Done);
        Ok(())
    });
    let display = show(receiver, Arc::clone(&stop));
    stop.store(true, Ordering::Relaxed);
    let result = worker
        .join()
        .map_err(|_| anyhow::anyhow!("native worker panicked"))?;
    display?;
    result
}
