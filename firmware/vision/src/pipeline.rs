use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use esp_idf_svc::hal::cpu::Core;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::task::thread::ThreadSpawnConfiguration;
use vision_core::config::VisionConfig;
use vision_core::enroll::{EnrollError, Enrollment, Step, Target};
use vision_core::gallery::best_matches;
use vision_core::people::{People, PeopleError, Role, MAX_NAME};
use vision_core::rect::Rect;
use vision_core::schedule::next_to_recognize;
use vision_core::stream::{
    encode_header, frame_header_len, StreamFace, StreamHeader, FLAG_ENROLLING, FLAG_RECOGNITION,
    MAX_STREAM_FACES,
};
use vision_core::track::Tracker;

use crate::camera::Camera;
use crate::face::{Detection, Detector, Embedder, Image};
use crate::storage::Storage;

pub enum Request {
    Enroll { name: String, role: Role },
    AddSample { id: u16 },
    Remove { id: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnrollStatus {
    Idle,
    Running,
    Done { id: u16 },
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonSummary {
    pub id: u16,
    pub name: String,
    pub role: Role,
    pub samples: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub fps: f32,
    pub capture_ms: u32,
    pub encode_ms: u32,
    pub detect_ms: u32,
    pub embed_ms: u32,
    pub faces: usize,
    pub tracks: usize,
    pub luma: u8,
    pub jpeg_bytes: usize,
}

pub struct Shared {
    frame: Mutex<(u32, Option<Arc<Vec<u8>>>)>,
    frame_ready: Condvar,
    pub people: Mutex<Vec<PersonSummary>>,
    pub enroll: Mutex<EnrollStatus>,
    pub stats: Mutex<Stats>,
    pub card: bool,
    viewers: AtomicUsize,
}

impl Shared {
    pub fn next_frame(&self, after: u32, timeout: Duration) -> Option<(u32, Arc<Vec<u8>>)> {
        let guard = self.frame.lock().unwrap();
        let (guard, _) = self
            .frame_ready
            .wait_timeout_while(guard, timeout, |(seq, _)| *seq == after)
            .unwrap();
        guard.1.clone().map(|frame| (guard.0, frame))
    }

    pub fn viewers(&self) -> usize {
        self.viewers.load(Ordering::Relaxed)
    }

    pub fn set_viewers(&self, viewers: usize) {
        self.viewers.store(viewers, Ordering::Relaxed);
    }

    fn publish(&self, frame: Vec<u8>) {
        let mut guard = self.frame.lock().unwrap();
        guard.0 = guard.0.wrapping_add(1);
        guard.1 = Some(Arc::new(frame));
        self.frame_ready.notify_all();
    }
}

pub struct Vision {
    pub camera: Camera,
    pub detector: Detector,
    pub embedder: Embedder,
    pub storage: Option<Storage>,
    pub people: People,
    pub config: VisionConfig,
}

const STACK_SIZE: usize = 24 * 1024;
const JPEG_QUALITY: u8 = 70;

pub fn start(vision: Vision, requests: Receiver<Request>) -> anyhow::Result<Arc<Shared>> {
    let shared = Arc::new(Shared {
        frame: Mutex::new((0, None)),
        frame_ready: Condvar::new(),
        people: Mutex::new(summarize(&vision.people)),
        enroll: Mutex::new(EnrollStatus::Idle),
        stats: Mutex::new(Stats::default()),
        card: vision.storage.is_some(),
        viewers: AtomicUsize::new(0),
    });
    ThreadSpawnConfiguration {
        name: Some(c"vision"),
        stack_size: STACK_SIZE,
        pin_to_core: Some(Core::Core1),
        ..Default::default()
    }
    .set()?;
    let worker = shared.clone();
    let spawned = thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || Pipeline::new(vision, worker).run(requests));
    ThreadSpawnConfiguration::default().set()?;
    spawned?;
    Ok(shared)
}

struct Pipeline {
    vision: Vision,
    shared: Arc<Shared>,
    tracker: Tracker,
    enrollment: Option<Enrollment>,
    embedding: Vec<f32>,
    image: Image,
    boot: Instant,
    seq: u16,
}

impl Pipeline {
    fn new(vision: Vision, shared: Arc<Shared>) -> Self {
        let embedding = vec![0.0; vision.embedder.len()];
        Self {
            vision,
            shared,
            tracker: Tracker::default(),
            enrollment: None,
            embedding,
            image: Image::default(),
            boot: Instant::now(),
            seq: 0,
        }
    }

    fn now_ms(&self) -> u32 {
        self.boot.elapsed().as_millis() as u32
    }

    fn run(mut self, requests: Receiver<Request>) {
        loop {
            loop {
                match requests.try_recv() {
                    Ok(request) => self.handle(request),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => return,
                }
            }
            if let Err(error) = self.frame() {
                log::warn!("vision frame: {error:#}");
                thread::sleep(Duration::from_millis(100));
            }
            // Lets core 1's idle task run, which feeds the task watchdog.
            FreeRtos::delay_ms(1);
        }
    }

    fn handle(&mut self, request: Request) {
        let target = match request {
            Request::Enroll { name, role } => {
                match heapless::String::<MAX_NAME>::try_from(name.trim()) {
                    Ok(name) if !name.is_empty() => Target::New { name, role },
                    _ => return self.fail("name must be 1 to 24 characters"),
                }
            }
            Request::AddSample { id } if self.vision.people.get(id).is_none() => {
                return self.fail("no such person")
            }
            Request::AddSample { id } => Target::Existing(id),
            Request::Remove { id } => return self.remove(id),
        };
        if self.vision.storage.is_none() {
            return self.fail("no SD card");
        }
        if self.enrollment.is_some() {
            return self.fail("an enrollment is already running");
        }
        self.enrollment = Some(Enrollment::new(target, self.now_ms()));
        *self.shared.enroll.lock().unwrap() = EnrollStatus::Running;
    }

    fn remove(&mut self, id: u16) {
        let Some(storage) = &self.vision.storage else {
            return;
        };
        if self.vision.people.remove(id).is_err() {
            return;
        }
        if let Err(error) = storage.save_people(&self.vision.people) {
            log::error!("saving people: {error:#}");
        }
        storage.remove_snapshot(id);
        self.tracker = Tracker::default();
        self.publish_people();
    }

    fn fail(&mut self, reason: &str) {
        log::warn!("enrollment failed: {reason}");
        self.enrollment = None;
        *self.shared.enroll.lock().unwrap() = EnrollStatus::Failed {
            reason: reason.into(),
        };
    }

    fn frame(&mut self) -> anyhow::Result<()> {
        let start = Instant::now();
        let frame = self
            .vision
            .camera
            .capture()
            .ok_or_else(|| anyhow::anyhow!("no camera frame"))?;
        self.image.copy_from(&frame);
        drop(frame);
        let image = core::mem::take(&mut self.image);
        let result = self.process(&image, start);
        self.image = image;
        result
    }

    fn process(&mut self, image: &Image, start: Instant) -> anyhow::Result<()> {
        let config = self.vision.config;
        let captured = start.elapsed();
        let faces: heapless::Vec<Detection, { crate::face::MAX_FACES }> = self
            .vision
            .detector
            .detect(image)
            .into_iter()
            .filter(|face| face.score >= config.min_face_score)
            .collect();
        let rects: heapless::Vec<Rect, { crate::face::MAX_FACES }> =
            faces.iter().map(Detection::rect).collect();
        self.tracker.update(&rects, &config);
        let detected = start.elapsed();

        let now = self.now_ms();
        if self.enrollment.is_some() {
            self.enroll_step(image, &faces, now)?;
        } else if !self.vision.people.is_empty() {
            if let Some(index) = next_to_recognize(self.tracker.tracks(), now, &config) {
                let track = &mut self.tracker.tracks_mut()[index];
                let face = &faces[track.detection.expect("scheduled tracks are detected")];
                self.vision
                    .embedder
                    .embed(image, face, &mut self.embedding)?;
                let matches = best_matches(&self.vision.people, &self.embedding);
                track.identity.observe(&matches, &config);
                track.recognized_ms = Some(now);
            }
        }
        let embedded = start.elapsed();

        let mut jpeg_bytes = 0;
        if self.shared.viewers() > 0 {
            let jpeg = image.encode_jpeg(JPEG_QUALITY)?;
            jpeg_bytes = jpeg.len();
            let message = self.encode(image, &jpeg);
            self.shared.publish(message);
        }
        let encoded = start.elapsed();
        self.record(captured, detected, embedded, encoded, faces.len());
        let mut stats = self.shared.stats.lock().unwrap();
        stats.luma = image.mean_luma();
        if jpeg_bytes > 0 {
            stats.jpeg_bytes = jpeg_bytes;
        }
        Ok(())
    }

    fn enroll_step(&mut self, image: &Image, faces: &[Detection], now: u32) -> anyhow::Result<()> {
        let config = self.vision.config;
        let Some(enrollment) = &mut self.enrollment else {
            return Ok(());
        };
        match enrollment.step(faces.len(), now, &config) {
            Ok(Step::Wait) => Ok(()),
            Err(error) => {
                self.fail(describe(error, &self.vision.people).as_str());
                Ok(())
            }
            Ok(Step::Embed) => {
                self.vision
                    .embedder
                    .embed(image, &faces[0], &mut self.embedding)?;
                let matches = best_matches(&self.vision.people, &self.embedding);
                match enrollment.add(self.embedding.clone(), &matches, &config) {
                    Ok(None) => Ok(()),
                    Ok(Some(samples)) => {
                        let target = enrollment.target.clone();
                        self.enrollment = None;
                        match self.commit(target, samples, image) {
                            Ok(id) => {
                                *self.shared.enroll.lock().unwrap() = EnrollStatus::Done { id };
                                self.publish_people();
                            }
                            Err(error) => self.fail(&format!("{error:#}")),
                        }
                        Ok(())
                    }
                    Err(error) => {
                        self.fail(describe(error, &self.vision.people).as_str());
                        Ok(())
                    }
                }
            }
        }
    }

    fn commit(
        &mut self,
        target: Target,
        samples: Vec<Vec<f32>>,
        image: &Image,
    ) -> anyhow::Result<u16> {
        let storage = self
            .vision
            .storage
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no SD card"))?;
        let people = &mut self.vision.people;
        let mut samples = samples.into_iter();
        let id = match target {
            Target::New { name, role } => {
                let first = samples.next().expect("enrollment yields samples");
                people.add(&name, role, first).map_err(people_error)?
            }
            Target::Existing(id) => id,
        };
        for sample in samples {
            people.add_sample(id, sample).map_err(people_error)?;
        }
        storage.save_people(people)?;
        storage.save_snapshot(id, &image.encode_jpeg(JPEG_QUALITY)?)?;
        log::info!("enrolled person {id}");
        Ok(id)
    }

    fn encode(&mut self, image: &Image, jpeg: &[u8]) -> Vec<u8> {
        let faces: heapless::Vec<StreamFace, MAX_STREAM_FACES> = self
            .tracker
            .tracks()
            .iter()
            .filter(|track| track.detection.is_some())
            .map(|track| StreamFace {
                track: track.id,
                class: track.identity.class(),
                confidence: track.identity.confidence_percent(),
                rect: track.rect,
            })
            .collect();
        let mut flags = 0;
        if self.vision.storage.is_some() && !self.vision.people.is_empty() {
            flags |= FLAG_RECOGNITION;
        }
        if self.enrollment.is_some() {
            flags |= FLAG_ENROLLING;
        }
        self.seq = self.seq.wrapping_add(1);
        let header = StreamHeader {
            flags,
            seq: self.seq,
            width: image.width(),
            height: image.height(),
        };
        let mut message = vec![0; frame_header_len(faces.len()) + jpeg.len()];
        let len = encode_header(&header, &faces, &mut message);
        message[len..].copy_from_slice(jpeg);
        message
    }

    fn record(
        &self,
        captured: Duration,
        detected: Duration,
        embedded: Duration,
        encoded: Duration,
        faces: usize,
    ) {
        let ms = |d: Duration| d.as_millis() as u32;
        let mut stats = self.shared.stats.lock().unwrap();
        let frame_fps = 1000.0 / ms(encoded).max(1) as f32;
        stats.fps = if stats.fps == 0.0 {
            frame_fps
        } else {
            stats.fps * 0.9 + frame_fps * 0.1
        };
        stats.capture_ms = ms(captured);
        stats.detect_ms = ms(detected - captured);
        stats.embed_ms = ms(embedded - detected);
        stats.encode_ms = ms(encoded - embedded);
        stats.faces = faces;
        stats.tracks = self.tracker.tracks().len();
    }

    fn publish_people(&self) {
        *self.shared.people.lock().unwrap() = summarize(&self.vision.people);
    }
}

fn summarize(people: &People) -> Vec<PersonSummary> {
    people
        .iter()
        .map(|person| PersonSummary {
            id: person.id,
            name: person.name.as_str().into(),
            role: person.role,
            samples: person.samples.len(),
        })
        .collect()
}

fn describe(error: EnrollError, people: &People) -> String {
    match error {
        EnrollError::Timeout => "no single face in view, try again".into(),
        EnrollError::AlreadyEnrolled(id) => match people.get(id) {
            Some(person) => format!("already enrolled as {}", person.name),
            None => "already enrolled".into(),
        },
        EnrollError::DifferentPerson => "that face does not match this person".into(),
    }
}

fn people_error(error: PeopleError) -> anyhow::Error {
    anyhow::anyhow!(match error {
        PeopleError::Full => "the people list is full",
        PeopleError::InvalidName => "name must be 1 to 24 characters",
        PeopleError::UnknownPerson => "no such person",
        PeopleError::SamplesFull => "this person has the maximum number of samples",
        PeopleError::WrongEmbeddingLength => "face model mismatch",
        PeopleError::Corrupt => "people file is corrupt",
    })
}
