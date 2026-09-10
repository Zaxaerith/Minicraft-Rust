//! Windows shared-mode rendering. The engine owns buffer sizing and resampling;
//! the worker writes only the free frames reported by GetCurrentPadding.

use std::{
    sync::{Arc, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0},
        Media::Audio::{
            AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM, AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY, IAudioClient, IAudioRenderClient,
            IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX, eConsole, eRender,
        },
        System::{
            Com::{
                CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
            },
            Threading::{
                AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, CreateEventW,
                INFINITE, SetEvent, WaitForSingleObject,
            },
        },
    },
    core::w,
};

use super::{PcmCommand as Command, PcmStream, SoundEffect, device_write_frames};

const RATE: u32 = 44_100;
const CHANNELS: usize = 2;

pub struct Backend(Option<Stream>);

impl Backend {
    pub fn new() -> Self {
        match Stream::new(false) {
            Ok(stream) => Self(Some(stream)),
            Err(error) => {
                eprintln!("Windows audio unavailable: {error}");
                Self(None)
            }
        }
    }

    pub fn play(&self, effect: SoundEffect) {
        if let Some(stream) = &self.0 {
            let _ = stream.send(Command::Play(effect));
        }
    }
}

// HANDLE is stored as an integer because a kernel event may be signaled from
// any thread. Arc keeps it alive until both the sender and worker have exited.
struct Event(usize);

impl Event {
    fn new() -> Result<Self, String> {
        unsafe { CreateEventW(None, false, false, None) }
            .map(|handle| Self(handle.0 as usize))
            .map_err(|error| error.to_string())
    }

    fn handle(&self) -> HANDLE {
        HANDLE(self.0 as *mut _)
    }

    fn signal(&self) -> Result<(), String> {
        unsafe { SetEvent(self.handle()) }.map_err(|error| error.to_string())
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.handle()) };
    }
}

struct ComGuard;

impl ComGuard {
    fn new() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map(|_| Self)
            .map_err(|error| error.to_string())
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        unsafe { CoUninitialize() };
    }
}

struct MultimediaPriority(Option<HANDLE>);

impl MultimediaPriority {
    fn new() -> Self {
        let mut index = 0;
        Self(unsafe { AvSetMmThreadCharacteristicsW(w!("Audio"), &mut index) }.ok())
    }
}

impl Drop for MultimediaPriority {
    fn drop(&mut self) {
        if let Some(handle) = self.0 {
            let _ = unsafe { AvRevertMmThreadCharacteristics(handle) };
        }
    }
}

#[derive(Debug, Default)]
struct DeviceReport {
    buffer_frames: u32,
    period_hns: i64,
    submitted_frames: u64,
    writes: u64,
    empty_padding: u64,
    startup_empty_padding: u64,
    max_service_gap_us: u64,
    play_commands: u64,
    multimedia_priority: bool,
}

struct Stream {
    sender: mpsc::Sender<Command>,
    event: Arc<Event>,
    worker: Option<JoinHandle<Result<DeviceReport, String>>>,
}

impl Stream {
    fn new(silent_probe: bool) -> Result<Self, String> {
        let event = Arc::new(Event::new()?);
        let worker_event = Arc::clone(&event);
        let (sender, pcm) = PcmStream::channel()?;
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("minicraft-audio".to_owned())
            .spawn(move || run(pcm, worker_event, ready_sender, silent_probe))
            .map_err(|error| error.to_string())?;
        match ready_receiver.recv() {
            Ok(Ok(())) => Ok(Self {
                sender,
                event,
                worker: Some(worker),
            }),
            result => {
                let _ = worker.join();
                Err(match result {
                    Ok(Err(error)) => error,
                    Err(error) => error.to_string(),
                    Ok(Ok(())) => unreachable!(),
                })
            }
        }
    }

    fn send(&self, command: Command) -> Result<(), String> {
        self.sender
            .send(command)
            .map_err(|error| error.to_string())?;
        self.event.signal()
    }

    fn finish(&mut self) -> Result<DeviceReport, String> {
        let _ = self.send(Command::Close);
        self.worker
            .take()
            .ok_or("audio worker already joined")?
            .join()
            .map_err(|_| "audio worker panicked".to_owned())?
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if self.worker.is_some() {
            let _ = self.finish();
        }
    }
}

struct Device {
    client: IAudioClient,
    render: IAudioRenderClient,
    frames: u32,
    period: i64,
}

impl Device {
    fn open(event: &Event) -> windows::core::Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let endpoint = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            let client: IAudioClient = endpoint.Activate(CLSCTX_ALL, None)?;
            // Keep the Java sample domain unchanged. Windows converts this stream
            // to the endpoint's actual rate/channel layout (e.g. 48 kHz float).
            let format = WAVEFORMATEX {
                wFormatTag: 1,
                nChannels: CHANNELS as u16,
                nSamplesPerSec: RATE,
                nAvgBytesPerSec: RATE * 4,
                nBlockAlign: 4,
                wBitsPerSample: 16,
                cbSize: 0,
            };
            client.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                    | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM
                    | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                0,
                0,
                &format,
                None,
            )?;
            client.SetEventHandle(event.handle())?;
            let frames = client.GetBufferSize()?;
            let mut period = 0;
            client.GetDevicePeriod(Some(&mut period), None)?;
            let render = client.GetService()?;
            Ok(Self {
                client,
                render,
                frames,
                period,
            })
        }
    }

    fn prime(&self) -> windows::core::Result<()> {
        unsafe {
            self.render.GetBuffer(self.frames)?;
            self.render
                .ReleaseBuffer(self.frames, AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)?;
            self.client.Start()
        }
    }

    fn refill(
        &self,
        pcm: &mut PcmStream,
        silent: bool,
        report: &mut DeviceReport,
    ) -> windows::core::Result<bool> {
        unsafe {
            let padding = self.client.GetCurrentPadding()?;
            let available = device_write_frames(self.frames, padding);
            if available == 0 {
                return Ok(false);
            }
            if padding == 0 {
                if report.writes == 0 {
                    report.startup_empty_padding += 1;
                } else {
                    report.empty_padding += 1;
                }
            }
            let buffer = self.render.GetBuffer(available)?;
            // GetBuffer/ReleaseBuffer are paired on this one worker; no stale
            // application-owned pointers can be replayed or refilled in flight.
            pcm.fill(std::slice::from_raw_parts_mut(
                buffer.cast::<i16>(),
                available as usize * CHANNELS,
            ));
            let flags = if silent {
                AUDCLNT_BUFFERFLAGS_SILENT.0 as u32
            } else {
                0
            };
            self.render.ReleaseBuffer(available, flags)?;
            report.submitted_frames += u64::from(available);
            report.writes += 1;
            Ok(true)
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = unsafe { self.client.Stop() };
        let _ = unsafe { self.client.Reset() };
    }
}

fn run(
    mut pcm: PcmStream,
    event: Arc<Event>,
    ready: mpsc::SyncSender<Result<(), String>>,
    silent: bool,
) -> Result<DeviceReport, String> {
    let initialization = (|| {
        let com = ComGuard::new()?;
        let priority = MultimediaPriority::new();
        let device = Device::open(&event).map_err(|error| error.to_string())?;
        device.prime().map_err(|error| error.to_string())?;
        Ok::<_, String>((com, priority, device))
    })();
    let (_com, priority, device) = match initialization {
        Ok(parts) => parts,
        Err(error) => {
            let _ = ready.send(Err(error.clone()));
            return Err(error);
        }
    };
    let mut report = DeviceReport {
        buffer_frames: device.frames,
        period_hns: device.period,
        multimedia_priority: priority.0.is_some(),
        ..DeviceReport::default()
    };
    let _ = ready.send(Ok(()));
    let mut last_write = Instant::now();
    loop {
        if unsafe { WaitForSingleObject(event.handle(), INFINITE) } != WAIT_OBJECT_0 {
            return Err("audio event wait failed".to_owned());
        }
        let running = pcm.poll_commands();
        report.play_commands = pcm.accepted_plays;
        if !running {
            return Ok(report);
        }
        if device
            .refill(&mut pcm, silent, &mut report)
            .map_err(|error| error.to_string())?
        {
            report.max_service_gap_us = report
                .max_service_gap_us
                .max(last_write.elapsed().as_micros() as u64);
            last_write = Instant::now();
        }
    }
}

/// Exercise real engine timing plus all ten decoders/mixing paths without sound.
pub fn device_probe() -> Result<(), String> {
    let mut stream = Stream::new(true)?;
    for _ in 0..3 {
        for effect in SoundEffect::ALL {
            stream.send(Command::Play(effect))?;
            thread::sleep(Duration::from_millis(30));
        }
    }
    thread::sleep(Duration::from_secs(2));
    let report = stream.finish()?;
    println!(
        "WASAPI probe: buffer_frames={}, period_ms={:.2}, writes={}, submitted_frames={}, startup_empty_padding={}, empty_padding={}, max_service_gap_us={}, play_commands={}, mmcss={}",
        report.buffer_frames,
        report.period_hns as f64 / 10_000.0,
        report.writes,
        report.submitted_frames,
        report.startup_empty_padding,
        report.empty_padding,
        report.max_service_gap_us,
        report.play_commands,
        report.multimedia_priority
    );
    if report.writes == 0 || report.play_commands != 30 {
        return Err("WASAPI probe did not consume every test command".to_owned());
    }
    if report.empty_padding != 0 {
        return Err("WASAPI probe observed an empty queue after startup".to_owned());
    }
    Ok(())
}
