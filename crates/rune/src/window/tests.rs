use super::*;

#[test]
fn updates_that_change_nothing_ask_no_window_for_a_frame() {
    use crate::{App, AppSpec, el};
    let app = App::new(AppSpec::new("t", (10.0, 10.0), || 0u32, |_, _| el()));
    let (handle, shared) = (app.handle(), app.shared.clone());
    let mut rt = Runtime::new(0u32, |_, _| el(), (10.0, 10.0), crate::Env::default());
    rt.frame(0.0);
    // As `Host::drain`, for 100 timer ticks that found nothing new.
    for _ in 0..100 {
        handle.update_if(|_, _| false);
        let mut requests = shared.take();
        let mut updated = false;
        for f in std::mem::take(&mut requests.updates) { updated |= rt.update_if(f); }
        let quiet = !updated && requests.only_updates() && !rt.patch_pending() && !rt.needs_commit();
        for (launcher, visible) in [(false, true), (true, true), (true, false), (false, false)] {
            let work = WakeWork::of(WakeFacts { window: true, launcher, visible, was_shown: visible, changed: updated, quiet, ..WakeFacts::default() }, || false);
            assert_eq!(work, WakeWork::Nothing, "launcher {launcher} visible {visible}");
        }
    }
    handle.update_if(|_, _| false);
    handle.wake();
    assert!(!shared.take().only_updates(), "`wake` still runs a shown window's poll");
    handle.update_if(|_, _| false);
    setup_waker(&shared)();
    assert!(!shared.take().only_updates(), "so does `setup`'s waker");
    assert_eq!(WakeWork::of(WakeFacts { window: true, visible: true, was_shown: true, ..WakeFacts::default() }, || false), WakeWork::Paint);
    assert_eq!(WakeWork::of(WakeFacts { window: true, launcher: true, changed: true, ..WakeFacts::default() }, || false), WakeWork::HiddenFrame);
}

/// A stream of updates that change nothing, faster than a frame, on a host that neither polls
/// nor draws for them, doesn't push back a due timer, and one's own `wake_after(ZERO)` after
/// idle is due at once.
#[test]
fn updates_that_change_nothing_do_not_delay_due_timers() {
    let mut rt = Runtime::new(0u32, |_, _| crate::el(), (10.0, 10.0), crate::Env::default());
    rt.wall_start = Some(Instant::now());
    rt.frame(0.0);
    // A second idle since that frame.
    rt.wall_start = rt.wall_start.map(|t| t - Duration::from_secs(1));
    let polled = rt.polled_at();
    rt.update_if(|_, tx| { tx.wake_after(Duration::ZERO); false });
    rt.schedule_pending_wakes();
    let due = timer_deadline(&rt).expect("armed");
    assert!(due <= Instant::now(), "due now, not a frame interval from now");
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(2));
        rt.update_if(|_, _| false);
        rt.schedule_pending_wakes();
        assert_eq!(timer_deadline(&rt), Some(due));
    }
    assert_eq!(rt.polled_at(), polled, "the clock stays at the last poll");
}

/// `wake_after(Duration::ZERO)` from the poll: a running display link takes it as the next
/// frame; any host without one waits a frame interval from the poll, never `now`.
#[test]
fn a_timer_asking_for_the_next_frame_waits_a_frame_without_a_display_link() {
    fn poll(polls: &mut u32, _: f64, tx: &mut Tx) -> bool { *polls += 1; tx.wake_after(Duration::ZERO); false }
    let mut rt = Runtime::new(0u32, |_, _| crate::el(), (10.0, 10.0), crate::Env::default());
    rt.poll = Some(poll);
    let start = Instant::now() - Duration::from_secs(1);
    rt.wall_start = Some(start);
    rt.frame(0.0);
    assert_eq!(rt.state, 1);
    let due = wall_wake_deadline(&rt).expect("armed");
    let polled = start + Duration::from_secs_f64(rt.polled_at());
    assert!(due <= polled + Duration::from_micros(1), "due at once on the runtime's clock");
    let wait = timer_deadline(&rt).expect("armed");
    assert!(wait >= polled + TIMER_FRAME - Duration::from_micros(1) && wait <= polled + TIMER_FRAME + Duration::from_micros(1));
    assert!(timer_wants_frame(&rt, Some(polled + Duration::from_millis(8))), "a paced surface draws it next tick");
    assert!(!timer_wants_frame(&rt, None), "an unpaced one doesn't keep the loop turning");
    // A timer longer than a frame keeps its own time.
    let mut rt = Runtime::new(0u32, |_, _| crate::el(), (10.0, 10.0), crate::Env::default());
    rt.wall_start = Some(start);
    rt.update(|_, tx| tx.wake_after(Duration::from_millis(100)));
    rt.frame(0.0);
    assert_eq!(timer_deadline(&rt), wall_wake_deadline(&rt));
}

#[test]
fn a_frame_that_draws_the_same_pixels_is_the_same() {
    use crate::{Color, el};
    fn view(n: &u32, _: &mut crate::Cx<u32>) -> crate::El {
        el().key("root").size(200.0, 100.0).child(el().key("box").size(40.0 + *n as f32, 40.0).fill(Color { r: 0.3, g: 0.5, b: 0.9, a: 1.0 }))
    }
    let mut rt = Runtime::new(0u32, view, (200.0, 100.0), crate::Env::new(false, 2.0));
    rt.frame(0.0);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 400,
        height: 200,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        color_space: Default::default(),
    };
    let mut presented = None;
    Presented::record(&mut presented, &rt, &config);
    let p = presented.as_ref().unwrap();

    // A poll that says it changed state, but the view draws the same.
    rt.update(|_, _| {});
    rt.frame(0.0);
    assert!(p.same(&rt, &config), "a rebuild that draws the same pixels presents nothing");

    let resized = wgpu::SurfaceConfiguration { width: 402, ..config.clone() };
    assert!(!p.same(&rt, &resized), "a resized surface draws again");
    rt.text.atlas.dirty = Some((0, 1));
    assert!(!p.same(&rt, &config), "a glyph upload draws again");
    rt.text.atlas.dirty = None;

    rt.update(|n, _| *n = 8);
    rt.settle_all();
    rt.frame(0.0);
    assert!(!p.same(&rt, &config), "a change on screen draws again");
}

/// A timer frame that draws the same pixels still takes its drawable while a poll steps on the
/// display link (`wake_after(ZERO)`): skipping it would stop the link and the steps with it.
#[test]
fn a_timer_frame_that_draws_the_same_still_draws_while_a_poll_steps() {
    fn poll(step: &mut bool, _: f64, tx: &mut Tx) -> bool {
        tx.wake_after(if *step { Duration::ZERO } else { Duration::from_secs(5) });
        false
    }
    let mut rt = Runtime::new(false, |_, _| crate::el().size(20.0, 20.0), (20.0, 20.0), crate::Env::default());
    rt.poll = Some(poll);
    rt.wall_start = Some(Instant::now());
    rt.frame(0.0);
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 20,
        height: 20,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        color_space: Default::default(),
    };
    let mut presented = None;
    Presented::record(&mut presented, &rt, &config);
    let p = presented.as_ref().unwrap();
    let next_tick = Some(Instant::now() + Duration::from_millis(8));
    assert!(p.unchanged(&rt, &config, false, next_tick, None), "a slow timer leaves the drawable");
    rt.state = true;
    rt.frame(0.0);
    assert!(p.same(&rt, &config));
    assert!(!p.unchanged(&rt, &config, false, next_tick, None), "a step due by the next tick draws");
    assert!(p.unchanged(&rt, &config, false, None, None), "without a running link the timer paces it");
}

#[cfg(target_os = "macos")]
#[test]
fn launch_frames_hold_two_drawables_until_the_window_rests() {
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 1960,
        height: 1520,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
        color_space: Default::default(),
    };
    let t0 = Instant::now();
    let mut launch = LaunchDrawables::cap(&mut config);
    assert_eq!(config.desired_maximum_frame_latency, 1, "Metal makes latency + 1 drawables");
    assert_eq!(launch.settle(false, t0), None);
    assert_eq!(launch.settle(false, t0 + Duration::from_millis(500)), None, "frames back to back stay capped");
    assert_eq!(launch.settle(true, t0 + Duration::from_millis(600)), Some(2), "resting restores the configured latency");
    assert_eq!(launch.settle(true, t0 + Duration::from_millis(700)), None, "once");
    assert_eq!(launch.settle(false, t0 + Duration::from_secs(5)), None);

    // A window that never rests gets its third drawable after its first second on screen.
    config.desired_maximum_frame_latency = 2;
    let mut launch = LaunchDrawables::cap(&mut config);
    let shown = t0 + Duration::from_secs(3);
    assert_eq!(launch.settle(false, shown), None, "the second counts from the first frame on screen");
    assert_eq!(launch.settle(false, shown + Duration::from_millis(999)), None);
    assert_eq!(launch.settle(false, shown + LaunchDrawables::LAUNCH), Some(2));

    // Nothing to restore when the configuration already asks for two.
    config.desired_maximum_frame_latency = 1;
    let mut launch = LaunchDrawables::cap(&mut config);
    assert_eq!(config.desired_maximum_frame_latency, 1);
    assert_eq!(launch.settle(true, t0), None);
}

/// The poll's timer on the runtime's logical clock.
const COMP_PERIOD: Duration = Duration::from_secs(5);

fn still(polls: &mut u32, _: f64, tx: &mut Tx) -> bool { *polls += 1; tx.wake_after(COMP_PERIOD); false }
fn every_third(polls: &mut u32, _: f64, tx: &mut Tx) -> bool { *polls += 1; tx.wake_after(COMP_PERIOD); *polls % 3 == 0 }
fn changed(polls: &mut u32, _: f64, tx: &mut Tx) -> bool { *polls += 1; tx.wake_after(COMP_PERIOD); true }

/// A compositor whose `animating` and `redraws` follow shared flags, as one driven by app state.
struct FlagComp { animates: std::rc::Rc<std::cell::Cell<bool>>, redraws: std::rc::Rc<std::cell::Cell<bool>> }

impl crate::Compositor<u32> for FlagComp {
    fn render(&mut self, f: &mut crate::Frame<'_, u32>) { f.draw_ui(); }
    fn animating(&self) -> bool { self.animates.get() }
    fn redraws(&self, _: &Runtime<u32>) -> bool { self.redraws.get() }
}

/// A window as the host runs it, with or without a compositor, counting its frames.
struct CompHost {
    rt: Runtime<u32>,
    presented: Option<Presented>,
    config: wgpu::SurfaceConfiguration,
    idle: bool,
    comp: Option<Box<dyn crate::Compositor<u32>>>,
    /// Whether `comp` animates.
    comp_animates: std::rc::Rc<std::cell::Cell<bool>>,
    /// Whether `comp` would draw something new (`Compositor::redraws`); true unless a test says.
    comp_redraws: std::rc::Rc<std::cell::Cell<bool>>,
    frames: u32,
    /// Called where `draw` calls it.
    reports: crate::frame::encoded::FrameReports<u32>,
}

impl CompHost {
    /// The first frame draws before `poll` is set, so one timer is armed at a time.
    fn new(comp: Option<bool>, poll: fn(&mut u32, f64, &mut Tx) -> bool) -> Self {
        fn view(_: &u32, _: &mut crate::Cx<u32>) -> crate::El { crate::el().key("box").size(20.0, 20.0).fill(crate::Color::WHITE) }
        let mut rt = Runtime::new(0u32, view, (20.0, 20.0), crate::Env::default());
        rt.test_clock = Some(crate::Clock::manual(1.0 / crate::frame::clock::RESUME_STEP));
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: 20,
            height: 20,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            color_space: Default::default(),
        };
        let comp_animates = std::rc::Rc::new(std::cell::Cell::new(comp == Some(true)));
        let comp_redraws = std::rc::Rc::new(std::cell::Cell::new(true));
        let comp = comp.map(|_| Box::new(FlagComp { animates: comp_animates.clone(), redraws: comp_redraws.clone() }) as Box<dyn crate::Compositor<u32>>);
        let mut host = Self { rt, presented: None, config, idle: false, comp, comp_animates, comp_redraws, frames: 0, reports: Default::default() };
        host.frame(false);
        host.rt.poll = Some(poll);
        host.frame(false);
        host
    }

    /// As `draw`: a quiet frame (only a timer or a key asked for it) that draws what's on screen
    /// presents nothing, unless a compositor would draw something new. Returns whether it presented.
    fn frame(&mut self, quiet: bool) -> bool {
        let started = Instant::now();
        self.rt.test_clock.as_mut().unwrap().step();
        let animating = self.rt.frame(crate::frame::clock::RESUME_STEP);
        let update_ms = started.elapsed().as_secs_f64() * 1000.0;
        if quiet && !composing(self.comp.as_deref())
            && self.presented.as_ref().is_some_and(|p| p.unchanged(&self.rt, &self.config, animating, None, self.comp.as_deref())) {
            self.reports.unchanged(None, &self.rt);
            return false;
        }
        let moving = animating || composing(self.comp.as_deref());
        let encoded = self.reports.stamp();
        self.idle = Presented::settle(&mut self.presented, &self.rt, &self.config, moving, None, true);
        self.frames += 1;
        self.reports.drawn(encoded, None, started, update_ms);
        true
    }

    /// As `handle_window_event` for a key press: its handlers run in a frame only the key asked
    /// for (`TimerFrame::asked_by_key`). Returns whether it presented.
    fn key(&mut self) -> bool {
        let mods = crate::Mods::default();
        self.rt.physical_with_fallback(crate::PhysicalInput::Key { key: crate::PhysicalKeyCode::KeyA, mods, pressed: true, repeat: false },
            Some(crate::KeyEvent { key: crate::Key::Text("a".into()), mods }));
        self.frame(true)
    }

    /// As `new_events` at a due timer: a resting window polls first and asks for a frame only
    /// when that poll changed something. Returns whether it presented.
    fn timer_wake(&mut self) -> bool {
        let due = self.rt.wake_deadline().expect("a timer is armed");
        // Advance host time, not rt.clock: rests() still examines the last poll's time.
        let clock = self.rt.test_clock.as_mut().unwrap();
        clock.seek(clock.now().max(due));
        let rests = self.idle && !composing(self.comp.as_deref()) && self.presented.is_some();
        let frame = match rests.then(|| self.rt.resting_wake()).flatten() {
            Some(stale) => {
                let frame = stale || self.rt.stale();
                self.rt.resting_wake_frame(frame);
                frame
            }
            None => true,
        };
        frame && self.frame(true)
    }
}

#[test]
fn a_still_compositor_s_window_rests_and_polls_its_timers_first() {
    let mut host = CompHost::new(Some(false), still);
    assert!(host.idle && host.presented.is_some(), "a still compositor lets the window rest");
    let (frames, commits, polls) = (host.frames, host.rt.stats.commits, host.rt.state);
    for _ in 0..8 { assert!(!host.timer_wake(), "a poll that returns false builds and presents nothing"); }
    assert_eq!(host.frames, frames);
    assert_eq!(host.rt.stats.commits, commits, "and runs no view");
    assert_eq!(host.rt.state, polls + 8, "each timer still polls");
}

#[test]
fn a_still_compositor_s_poll_that_changes_something_gets_one_frame() {
    let mut host = CompHost::new(Some(false), every_third);
    let (frames, polls) = (host.frames, host.rt.state);
    let drew = (0..6).filter(|_| host.timer_wake()).count();
    let changes = (polls + 1..=polls + 6).filter(|p| p % 3 == 0).count();
    assert_eq!(drew, changes, "one frame for each poll that returns true");
    assert_eq!(host.frames - frames, changes as u32);
    assert_eq!(host.rt.state, polls + 6, "one poll a wake: the frame doesn't poll again");
}

#[test]
fn an_animating_compositor_draws_every_frame() {
    let mut host = CompHost::new(Some(true), still);
    assert!(!host.idle && host.presented.is_none(), "an animating compositor never rests");
    let (frames, polls) = (host.frames, host.rt.state);
    for _ in 0..5 { assert!(host.timer_wake()); }
    assert_eq!(host.frames - frames, 5);
    assert_eq!(host.rt.state, polls + 5, "the frames poll");
}

#[test]
fn a_compositor_that_redraws_never_counts_as_unchanged() {
    let mut host = CompHost::new(Some(false), still);
    host.rt.update(|_, _| {});
    host.rt.frame(crate::frame::clock::RESUME_STEP);
    let p = host.presented.as_ref().unwrap();
    assert!(p.same(&host.rt, &host.config));
    assert!(!p.unchanged(&host.rt, &host.config, false, None, host.comp.as_deref()), "the same instances may composite differently");
    // A changed poll's quiet frame presents.
    let mut host = CompHost::new(Some(false), changed);
    assert!(host.idle && host.presented.is_some());
    assert!(host.timer_wake(), "a poll that changed nothing on screen still presents a compositor's frame");

    // Without a compositor, the same frame presents nothing.
    let mut host = CompHost::new(None, still);
    host.rt.poll = Some(changed);
    let frames = host.frames;
    assert!(!host.timer_wake());
    assert_eq!(host.frames, frames);
}

/// A terminal's key handler: the key goes to the shell, and only its echo changes the screen.
fn sends_the_key_away(_: &mut u32, _: &crate::KeyEvent, tx: &mut Tx) -> bool { tx.live_only(); true }

#[test]
fn a_key_that_changes_nothing_on_screen_presents_no_frame() {
    let mut host = CompHost::new(Some(false), still);
    host.rt.on_key = Some(sends_the_key_away);
    host.rt.check_live_only(false);
    host.comp_redraws.set(false);
    let frames = host.frames;
    for _ in 0..5 { assert!(!host.key(), "the compositor draws nothing new, so the key's frame takes no drawable"); }
    assert_eq!(host.frames, frames);
    assert_eq!(host.rt.live_only_events(), 5, "the handler heard each key");
    assert!(host.idle && host.presented.is_some(), "the window still rests on the frame on screen");
    // The echo changes what the compositor draws: its frame presents.
    host.comp_redraws.set(true);
    assert!(host.frame(false));

    // A compositor that doesn't say presents the key's frame, as before.
    let mut host = CompHost::new(Some(false), still);
    host.rt.on_key = Some(sends_the_key_away);
    host.rt.check_live_only(false);
    assert!(host.key());

    // Without a compositor, a key whose frame builds the same instances presents nothing.
    let mut host = CompHost::new(None, still);
    host.rt.on_key = Some(sends_the_key_away);
    host.rt.check_live_only(false);
    assert!(!host.key());
}

#[test]
fn a_compositor_that_starts_animating_while_the_window_rests_draws_its_next_timer() {
    let mut host = CompHost::new(Some(false), still);
    assert!(!host.timer_wake());
    host.comp_animates.set(true);
    assert!(host.timer_wake(), "a due timer draws once the compositor animates");
    assert!(!host.idle && host.presented.is_none(), "and the window stops resting");
    host.comp_animates.set(false);
    assert!(host.timer_wake());
    assert!(!host.timer_wake(), "still again, it rests");
}

#[test]
fn a_frame_that_leaves_work_for_the_next_does_not_settle() {
    let mut host = CompHost::new(Some(false), still);
    assert!(host.idle);
    // The compositor invalidated the view while drawing, as one ending its own motion may.
    host.rt.invalidate();
    let idle = Presented::settle(&mut host.presented, &host.rt, &host.config, false, None, true);
    assert!(!idle && host.presented.is_none(), "a frame with a commit pending isn't idle");
    host.frame(false);
    assert!(host.idle && host.presented.is_some(), "the frame that commits it settles");
    // A frame the window may not rest on (hidden, or a trace counting frames) records nothing.
    let idle = Presented::settle(&mut host.presented, &host.rt, &host.config, false, None, false);
    assert!(idle && host.presented.is_none());
}

/// The same frames, timers and settling with `App::on_frame_encoded` as without: a window
/// reports what it decided and nothing it decides reads the reports.
#[test]
fn reporting_frames_changes_no_frame_skip_or_redraw() {
    fn run(hook: bool) -> (Vec<(bool, bool, bool, bool, u32)>, u64, u32, Vec<crate::FrameOutcome>) {
        let log = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = log.clone();
        let mut host = CompHost::new(None, every_third);
        let hook = hook.then(|| Box::new(move |f: &crate::FrameEncoded| sink.borrow_mut().push(f.outcome())) as crate::frame::encoded::OnFrameEncoded);
        host.reports = crate::frame::encoded::FrameReports::new(hook, Some("main"));
        let mut steps = Vec::new();
        for i in 0..12 {
            let presented = if i % 5 == 0 { host.frame(false) } else { host.timer_wake() };
            steps.push((presented, host.idle, host.rt.stale(), host.rt.wake_deadline().is_some(), host.frames));
        }
        let outcomes = log.borrow().clone();
        (steps, host.rt.stats.commits, host.rt.state, outcomes)
    }
    let (plain, plain_commits, plain_polls, none) = run(false);
    let (hooked, hooked_commits, hooked_polls, outcomes) = run(true);
    assert_eq!(plain, hooked);
    assert_eq!((plain_commits, plain_polls), (hooked_commits, hooked_polls));
    assert!(none.is_empty());
    assert!(outcomes.contains(&crate::FrameOutcome::Drawn) && outcomes.contains(&crate::FrameOutcome::Unchanged), "{outcomes:?}");
    let drawn = outcomes.iter().filter(|o| **o == crate::FrameOutcome::Drawn).count() as u32;
    assert_eq!(drawn, hooked.last().unwrap().4 - 2, "one Drawn per presented frame after the two `new` drew");
}

#[test]
fn hidden_pace_renders_a_change_at_once_then_at_most_every_refresh() {
    let t0 = Instant::now();
    let mut pace = HiddenPace::default();
    assert!(pace.changed(t0), "the first change renders at once");
    pace.rendered(t0);
    assert_eq!(pace.deadline(), None);
    assert!(!pace.changed(t0 + Duration::from_millis(100)));
    assert!(!pace.changed(t0 + Duration::from_millis(200)));
    assert_eq!(pace.deadline(), Some(t0 + HIDDEN_REFRESH), "later ones wait for one render");
    assert!(!pace.take_due(t0 + Duration::from_millis(249)));
    assert!(pace.take_due(t0 + HIDDEN_REFRESH));
    assert_eq!(pace.deadline(), None, "a due render asked for doesn't wake the loop again");
    assert!(pace.changed(t0 + HIDDEN_REFRESH));
    pace.rendered(t0 + HIDDEN_REFRESH);
    assert_eq!(pace.deadline(), None);
    assert!(pace.changed(t0 + Duration::from_secs(5)), "after a quiet spell, at once again");
    pace.reset();
    assert!(pace.changed(t0 + Duration::from_secs(5)));
}

#[test]
fn missed_frames_never_poll() {
    use wgpu::CurrentSurfaceTexture as S;
    let now = Instant::now();
    for status in [S::Occluded, S::Timeout, S::Outdated, S::Lost, S::Validation] {
        let missed = Missed::of(status).err().expect("a miss");
        for paced in [false, true] {
            for wait in [Duration::ZERO, OCCLUDED_RETRY, Duration::from_secs(60)] {
                let mut backoff = Backoff::seeded(wait, TIMEOUT_RETRY);
                backoff.step(BackoffEvent::Missed { missed, paced, now });
                // A retry deadline at or before now would spin like a poll.
                let flow = match backoff.deadline() {
                    Some(at) => { assert!(at > now, "{missed:?} retries later, not now"); ControlFlow::WaitUntil(at) }
                    None => ControlFlow::Wait,
                };
                assert_ne!(flow, ControlFlow::Poll, "{missed:?} (paced: {paced}) must not poll");
            }
        }
    }
    let occluded = |wait| {
        let mut backoff = Backoff::seeded(wait, TIMEOUT_RETRY);
        backoff.step(BackoffEvent::Missed { missed: Missed::Occluded, paced: true, now });
        backoff.deadline()
    };
    assert_eq!(occluded(Duration::ZERO), Some(now + OCCLUDED_RETRY), "the safety net is slow");
    assert_eq!(occluded(Duration::from_secs(60)), Some(now + OCCLUDED_MAX));
    let miss = |backoff: &mut Backoff, missed| {
        backoff.step(BackoffEvent::Missed { missed, paced: true, now });
        backoff.deadline()
    };
    let mut backoff = Backoff::default();
    assert_eq!(miss(&mut backoff, Missed::Timeout), None, "a running display link retries on its tick");
    assert_eq!(miss(&mut backoff, Missed::Outdated), Some(now + OCCLUDED_RETRY));
    assert_eq!(miss(&mut backoff, Missed::Occluded), Some(now + OCCLUDED_RETRY), "an occluded miss waits its backoff");
    assert_eq!(miss(&mut backoff, Missed::Occluded), Some(now + OCCLUDED_RETRY * 2), "and backs off");
    assert_eq!(miss(&mut backoff, Missed::Outdated), Some(now + OCCLUDED_RETRY), "other misses leave the backoff");
    assert_eq!(miss(&mut backoff, Missed::Occluded), Some(now + OCCLUDED_RETRY * 4), "which carries on");
}

/// A shown window's due timer asks for a frame. On Wayland a surface that is covered before its
/// frame callback fires withholds `RedrawRequested`, and the same overdue timer would turn the
/// loop at once for a frame that can't come. The host waits out the bound, then runs the timer
/// covered; its next one is ahead, so the loop sleeps.
#[test]
fn a_timer_frame_that_never_comes_runs_its_timer_covered_instead_of_spinning() {
    #[derive(Default)]
    struct Due { polls: u32 }
    fn poll(app: &mut Due, _: f64, tx: &mut Tx) -> bool {
        app.polls += 1;
        tx.wake_after(Duration::from_millis(100));
        true
    }
    let mut rt = Runtime::new(Due::default(), |_, _| crate::el().size(20.0, 20.0), (20.0, 20.0), crate::Env::default());
    rt.poll = Some(poll);
    rt.wall_start = Some(Instant::now());
    rt.frame(0.0);
    // A change since that frame, as a `wake` whose poll it hasn't drawn leaves it.
    rt.invalidate();
    assert!(rt.stale() && !rt.rests(), "the window doesn't rest: its frame is still to come");
    // As `new_events` at the timer, which is due by the time the loop runs for it.
    let due = timer_deadline(&rt).expect("a timer is armed");
    rt.wall_start = rt.wall_start.map(|t| t - due.saturating_duration_since(Instant::now()) - Duration::from_millis(1));
    let now = Instant::now();
    assert!(timer_deadline(&rt).expect("armed") <= now, "the timer is due: this is what turned the loop");

    let mut frame = TimerFrame::default();
    frame.asked_quiet(now);
    assert_eq!(frame.wait(timer_deadline(&rt), now), Some(now + TIMER_FRAME_STALL),
        "the frame the timer asked for keeps the loop asleep until the bound");
    assert!(!frame.stalled(now + TIMER_FRAME_STALL - Duration::from_millis(1)));
    assert!(frame.stalled(now + TIMER_FRAME_STALL), "after the bound the timer runs covered");

    let polls = rt.state.polls;
    covered_step(&mut rt);
    assert_eq!(rt.state.polls, polls + 1, "the due timer polled");
    let next = timer_deadline(&rt).expect("its next timer is armed");
    assert!(next > Instant::now(), "the next timer is ahead: {next:?}");
    // The covered step takes the frame's place, and a timer still ahead keeps its own time.
    frame.drawn();
    frame.asked_quiet(Instant::now());
    assert_eq!(frame.wait(timer_deadline(&rt), Instant::now()), Some(next), "the loop sleeps until its next timer");
    frame.drawn();
    assert!(!frame.stalled(now + TIMER_FRAME_STALL), "a frame that comes clears it");
}

#[test]
fn a_window_s_first_updates_can_register_on_a_later_window() {
    use crate::{App, AppSpec, MenuItem, el};
    type S = Vec<String>;
    let apps: Vec<App<S>> = ["first", "second"].map(|title| App::new(AppSpec::new(title, (10.0, 10.0), Vec::new, |_, _| el()))).into();
    let second = apps[1].handle();
    apps[0].handle().update(move |s: &mut S, _| {
        s.push(format!("{:?}", second.register_shortcut("ctrl+alt+shift+f18".parse().unwrap(), |_, _| {}).map(|_| ())));
        s.push(format!("{:?}", second.set_menu([MenuItem::new("Item", |_: &mut S, _| {})])));
    });
    // As Host::new and Host::resumed: attach the guards, then drain the first updates.
    let hosts = start_all(apps.into(), |_, app| {
        let shortcuts = crate::shortcuts::Guard::attach(&app.shared, Arc::new(|| {}), true, false);
        let menus = crate::native_menu::Guard::attach(&app.shared, true);
        Some((app, shortcuts, menus, S::new()))
    }, |(app, _, _, state)| for f in app.shared.take().updates { f(state, &mut Tx::default()); });
    assert_eq!(hosts[0].as_ref().unwrap().3, ["Ok(())", "Ok(())"]);
}

#[test]
fn drag_region_policy_preserves_titled_windows_and_panels() {
    for enabled in [false, true] {
        for full_size in [false, true] {
            let chrome = Chrome { drag_regions: enabled, full_size_content: full_size, ..Chrome::DEFAULT };
            assert_eq!(uses_drag_regions(chrome, false, false), enabled && full_size);
            assert_eq!(uses_drag_regions(chrome, true, false), enabled);
            assert!(!uses_drag_regions(chrome, false, true));
            assert!(!uses_drag_regions(chrome, true, true));
        }
    }
}

/// A GPU-free wgpu instance.
#[cfg(target_os = "linux")]
fn noop_instance() -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::NOOP;
    descriptor.backend_options.noop.enable = true;
    wgpu::Instance::new(descriptor)
}

/// A GPU on wgpu's noop backend: its device takes anything and refuses every surface.
#[cfg(target_os = "linux")]
fn noop_gpu(instance: &wgpu::Instance) -> (Gpu, wgpu::Surface<'static>) {
    use std::num::NonZeroU32;
    use std::ptr::NonNull;
    use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle, XcbDisplayHandle, XcbWindowHandle};
    // The noop backend takes any handle and reports no capabilities, so configuring fails.
    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::Xcb(XcbDisplayHandle::new(Some(NonNull::dangling()), 0))),
            raw_window_handle: RawWindowHandle::Xcb(XcbWindowHandle::new(NonZeroU32::new(1).unwrap())),
        })
    }
    .unwrap();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap();
    (Gpu::from_device(instance.clone(), adapter, device, queue), surface)
}

/// What the noop backend takes for a surface: the format the test configures it in.
#[cfg(target_os = "linux")]
fn noop_config() -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 200,
        height: 80,
        present_mode: wgpu::PresentMode::AutoVsync,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
        view_formats: vec![],
        color_space: Default::default(),
    }
}

/// A surface that refuses its configuration reports it as an error its owner can drop, not a
/// panic in wgpu's default handler that takes the app's every window with it (hex#979).
#[cfg(target_os = "linux")]
#[test]
fn a_refused_surface_configuration_is_an_error_not_a_panic() {
    let instance = noop_instance();
    let (gpu, surface) = noop_gpu(&instance);
    let config = noop_config();
    let error = configure(&surface, &gpu, &config).unwrap_err();
    assert!(error.starts_with("Validation Error: In Surface::configure"), "{error}");
    assert!(error.ends_with("(200x80, Bgra8UnormSrgb, PreMultiplied, AutoVsync)"), "{error}");
}

/// A surface whose configuration failed has no presentation, and wgpu panics
/// ("Surface is not configured for presentation") on the drawable Rune would ask it for. The
/// acquire reports the miss instead, and the window makes a surface and device before its next
/// frame (hex's Modern Standby panic).
#[cfg(target_os = "linux")]
#[test]
fn a_surface_with_no_presentation_is_never_asked_for_a_drawable() {
    let instance = noop_instance();
    let (gpu, surface) = noop_gpu(&instance);
    let config = noop_config();
    configure(&surface, &gpu, &config).expect_err("the noop surface refuses its configuration");
    let mut meter = SurfaceMeter::new("unconfigured", Arc::default());
    assert_eq!(acquire(&surface, &gpu, &config, false, 1, &mut meter), Err(Missed::Unconfigured));
    assert_eq!(meter.totals().failed(), 1, "the miss is counted");
}

/// `Device::destroy` only marks the device invalid: its lost callback runs when the device next
/// maintains itself, which `configure` does with a poll. Without that poll the surface gets no
/// presentation while `is_lost` still reads false, `configure` says Ok, and the drawable the next
/// frame asks for panics ("Surface is not configured for presentation").
#[cfg(target_os = "linux")]
#[test]
fn a_destroyed_device_that_has_not_reported_its_loss_is_not_configured() {
    let instance = noop_instance();
    let (gpu, surface) = noop_gpu(&instance);
    let config = noop_config();
    gpu.device.destroy();
    assert!(!gpu.is_lost(), "the callback waits for the device's next maintain");
    let error = configure(&surface, &gpu, &config).unwrap_err();
    assert!(error.starts_with("the GPU device was lost"), "{error}");
    assert!(gpu.is_lost());
    let mut meter = SurfaceMeter::new("destroyed", Arc::default());
    assert_eq!(acquire(&surface, &gpu, &config, true, 1, &mut meter), Err(Missed::Unconfigured));
}

/// A surface that won't come back is replaced a bounded number of times, waiting longer after
/// each failure, and then the window gives it up: a lost device can never be replaced, and a
/// frame must not ask again forever.
#[test]
fn surface_replacements_give_up_after_a_bounded_number_of_tries() {
    use super::win::{SurfaceStep, SURFACE_TRIES};
    let now = Instant::now();
    let mut retries = SurfaceRetries::default();
    let mut waits = Vec::new();
    loop {
        assert_eq!(retries.attempt(), SurfaceStep::Replace);
        match retries.retry(now) {
            Some(at) => waits.push(at - now),
            None => break,
        }
        assert!(waits.len() < 100, "a replacement that keeps failing never gives up");
    }
    assert_eq!(waits.len() as u8, SURFACE_TRIES - 1, "one try, then bounded retries");
    assert!(waits.windows(2).all(|pair| pair[1] > pair[0]), "each wait is longer: {waits:?}");
    assert_eq!(retries.failures(), SURFACE_TRIES);
    assert_eq!(retries.attempt(), SurfaceStep::GiveUp);

    // A frame that reached the screen starts over.
    retries.worked();
    assert_eq!(retries.failures(), 0);
    assert_eq!(retries.attempt(), SurfaceStep::Replace);
}

/// A replacement that works still counts: a surface replaced and lost again before its frame
/// presents gives the window up after `SURFACE_TRIES` replacements instead of making one
/// forever. This is the shape `Host::replace_lost_surface` runs, where only `draw`'s present
/// calls `worked`.
#[test]
fn a_surface_lost_after_every_replacement_gives_up_after_the_tries() {
    use super::win::{SurfaceStep, SURFACE_TRIES};
    let mut retries = SurfaceRetries::default();
    let mut replaced = 0;
    while retries.attempt() == SurfaceStep::Replace {
        // The replacement worked, and the frame it made was lost before it presented.
        replaced += 1;
        assert!(replaced < 100, "a surface lost before it presents never stops being replaced");
    }
    assert_eq!(replaced, SURFACE_TRIES, "the window gives up after a bounded number of replacements");

    // One frame reaches the screen: the next loss starts over.
    retries.worked();
    assert_eq!(retries.attempt(), SurfaceStep::Replace);
}

/// A device that went away (a driver reset, a GPU removed, Windows' Modern Standby) configures
/// nothing, and wgpu drops the `DeviceLost` that would say so: `configure` reports the lost
/// device itself, and no drawable is ever asked of it.
#[cfg(target_os = "linux")]
#[test]
fn a_lost_device_is_reported_and_never_drawn_with() {
    let instance = noop_instance();
    let (gpu, surface) = noop_gpu(&instance);
    let config = noop_config();
    assert!(!gpu.is_lost());
    gpu.device.destroy();
    // wgpu loses a destroyed device once its queues drain, when the device next maintains itself.
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).ok();
    assert!(gpu.is_lost(), "the device's lost callback marks it");
    let error = configure(&surface, &gpu, &config).unwrap_err();
    assert!(error.starts_with("the GPU device was lost"), "{error}");
    let mut meter = SurfaceMeter::new("lost", Arc::default());
    assert_eq!(acquire(&surface, &gpu, &config, true, 1, &mut meter), Err(Missed::Unconfigured));
    assert_eq!(meter.totals().failed(), 1, "a lost device is a miss, not a panic");
}

/// Each Rune GPU has its own wgpu instance, and wgpu compares devices by an id each instance
/// numbers from the start, so a new GPU's device equals the old one's. A renderer's device key
/// tells them apart, so a compositor keeping resources by it remakes them (hex#982).
#[cfg(target_os = "linux")]
#[test]
fn a_new_gpus_device_equals_the_old_one_but_its_key_differs() {
    let device = || {
        let adapter = pollster::block_on(noop_instance().request_adapter(&wgpu::RequestAdapterOptions::default())).unwrap();
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).unwrap().0
    };
    let (old, new) = (device(), device());
    assert!(old == new, "wgpu tells devices of separate instances apart by now: device equality may do");
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let (old, new) = (crate::render::Renderer::new(&old, format), crate::render::Renderer::new(&new, format));
    assert_ne!(old.device_key(), new.device_key());
}

/// `AppHandle::inspect` as `Host::drain` runs it: a press one inspection feeds the window is
/// handled before the next inspection reads it, with no frame between. The press asks the
/// window for a frame; the read asks for none.
#[test]
fn an_inspection_s_input_is_handled_before_the_next_inspection_reads() {
    use crate::{App, AppSpec, el};
    fn view(_: &u32, _: &mut crate::Cx<u32>) -> crate::El {
        el().key("button").w(40.0).h(20.0).on_press(|n: &mut u32| *n += 1)
    }
    let app = App::new(AppSpec::new("t", (100.0, 100.0), || 0u32, view));
    let (handle, shared) = (app.handle(), app.shared.clone());
    let mut rt = Runtime::new(0u32, view, (100.0, 100.0), crate::Env::default());
    rt.frame(0.0);
    let (read, seen) = std::sync::mpsc::channel();
    handle.inspect(|rt| {
        let (x, y) = rt.find("button").expect("laid out");
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
    });
    handle.inspect(move |rt| read.send(rt.state).unwrap());
    let mut inspections = shared.take().inspections.into_iter();
    assert!(lifecycle::inspect(&mut rt, inspections.next().unwrap()), "the press asks for a frame");
    assert!(!lifecycle::inspect(&mut rt, inspections.next().unwrap()), "the read asks for none");
    assert_eq!(seen.recv().unwrap(), 1, "the second inspection saw the press");
}

/// A covered window builds no view for an update until it shows again; an inspection builds it
/// first, so it reads the update, and asks for a frame to show it.
#[test]
fn an_inspection_reads_the_view_an_update_built_without_a_frame() {
    use crate::{App, AppSpec, el};
    fn view(n: &u32, _: &mut crate::Cx<u32>) -> crate::El {
        el().key(format!("count-{n}")).w(40.0).h(20.0)
    }
    let app = App::new(AppSpec::new("t", (100.0, 100.0), || 0u32, view));
    let (handle, shared) = (app.handle(), app.shared.clone());
    let mut rt = Runtime::new(0u32, view, (100.0, 100.0), crate::Env::default());
    rt.frame(0.0);
    rt.update(|n, _| *n += 1);
    let (read, seen) = std::sync::mpsc::channel();
    handle.inspect(move |rt| read.send(rt.find("count-1").is_some()).unwrap());
    handle.inspect(|_| {});
    let mut inspections = shared.take().inspections.into_iter();
    assert!(lifecycle::inspect(&mut rt, inspections.next().unwrap()), "the update asks for a frame to show it");
    assert!(seen.recv().unwrap(), "the inspection read the updated view");
    assert!(!lifecycle::inspect(&mut rt, inspections.next().unwrap()), "a read of a built view asks for none");
}

#[derive(Default)]
struct RightClicks { drags: Vec<crate::Phase>, menus: Vec<&'static str> }

/// The window's right button reaches an element that takes the secondary button as a press,
/// drag and release, and still opens the menu of one that does not.
#[test]
fn the_right_button_reaches_an_element_that_takes_it_and_opens_the_menu_of_one_that_does_not() {
    use crate::{PointerButton, el};
    use winit::event::{ElementState, MouseButton};
    let view = |_: &RightClicks, _: &mut crate::Cx<RightClicks>| {
        el().w_full().h_full()
            .child(el().key("takes").size(40.0, 40.0).pointer_buttons([PointerButton::Secondary])
                .on_drag(|s: &mut RightClicks, d: &mut crate::Drag, _: &mut crate::Tx| s.drags.push(d.phase)))
            .child(el().key("menu").size(40.0, 40.0).on_secondary_press(|s: &mut RightClicks, _, _| s.menus.push("menu")))
    };
    let mut rt = Runtime::new(RightClicks::default(), view, (100.0, 100.0), crate::Env::default());
    rt.frame(0.0);
    let (x, y) = rt.find("takes").unwrap();
    mouse_button(&mut rt, x + 5.0, y + 5.0, MouseButton::Right, ElementState::Pressed);
    rt.frame(0.0);
    mouse_button(&mut rt, x + 5.0, y + 5.0, MouseButton::Right, ElementState::Released);
    rt.frame(0.0);
    assert_eq!(rt.state.drags, [crate::Phase::Down, crate::Phase::Up]);
    assert!(rt.state.menus.is_empty());

    let (x, y) = rt.find("menu").unwrap();
    mouse_button(&mut rt, x + 5.0, y + 5.0, MouseButton::Right, ElementState::Pressed);
    mouse_button(&mut rt, x + 5.0, y + 5.0, MouseButton::Right, ElementState::Released);
    rt.frame(0.0);
    assert_eq!(rt.state.menus, ["menu"]);
}
