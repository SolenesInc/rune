//! The typed side of a runtime: the app's state, its view and its hooks. [`RuntimeCore`] reaches
//! it through [`AppSide`], so only this file's code is built once for each state type.

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use super::*;
use crate::playground::{Live, Moved, Nav, Region, Request};
use crate::states::{States, StatesRef};

/// The view context the core builds: [`Cx`] carries its state type only in a marker.
pub(crate) type CxAny = Cx<()>;

/// The app hooks the core asks about before it calls them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hook { Poll, Key, FieldKey, Focus, Mods, Visibility, Ime }

/// What the core needs of the app's state, view and hooks, whatever the state's type.
pub(crate) trait AppSide: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// The states a handler call can reach: the app's, or the playground stage's beside its controls.
    fn states(&mut self) -> States<'_>;
    fn states_ref(&self) -> StatesRef<'_>;
    /// The app's view, or with the playground attached, the view it composes. The context comes
    /// back with what the view set on it.
    fn root_view(&self, cx: CxAny, viewport: (f32, f32)) -> (El, CxAny);
    fn has(&self, hook: Hook) -> bool;
    fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool;
    /// `on_key` or `on_field_key`, given the stage's state while the playground shows one.
    fn key(&mut self, field: bool, ev: &KeyEvent, tx: &mut Tx) -> bool;
    fn focus(&mut self, focused: bool, tx: &mut Tx);
    fn visibility(&mut self, visible: bool, tx: &mut Tx);
    /// False when the playground shows a story with its own state.
    fn mods(&mut self, mods: crate::Mods, tx: &mut Tx) -> bool;
    fn ime(&mut self, ev: &Ime, tx: &mut Tx) -> bool;
    fn playground(&self) -> Option<&dyn LiveDyn>;
    fn playground_mut(&mut self) -> Option<&mut dyn LiveDyn>;
    fn playground_typing(&self) -> bool;
    fn playground_apply(&mut self, clock: f64) -> bool;
}

/// The playground as the core drives it.
pub(crate) trait LiveDyn {
    fn is_open(&self) -> bool;
    fn region(&self) -> Region;
    fn region_at(&self, x: f32, width: f32) -> Region;
    fn is_shortcut(&self, ev: &KeyEvent) -> bool;
    fn side_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool;
    fn nav(&self, ev: &KeyEvent, region: Region) -> Nav;
    fn clear_filter(&mut self) -> bool;
    fn first_shown(&self) -> Option<usize>;
    fn first_match(&self) -> Option<usize>;
    fn moved(&mut self, moved: Moved) -> bool;
    fn saw_focus(&self, focus: Option<&str>) -> bool;
    fn request(&mut self, requests: Vec<Request>);
    fn has_pending(&self) -> bool;
    fn pending_swaps_stage(&self) -> bool;
    fn pending_replays_only(&self) -> bool;
    fn polls(&self) -> bool;
    fn poll_stage(&mut self, t: f64, tx: &mut Tx) -> bool;
    fn take_pressed(&mut self) -> Vec<KeyEvent>;
    fn root(&self) -> usize;
    fn set_root(&mut self, root: usize);
    fn pressing(&self) -> bool;
    fn set_pressing(&mut self, pressing: bool);
    fn played(&self) -> u64;
    fn app_time_scale(&self) -> f32;
    fn set_app_time_scale(&mut self, scale: f32);
    fn open_at(&mut self, story: &str, variant: Option<&str>) -> Result<(), String>;
    fn trigger_or_why(&self, key: char) -> Result<Request, String>;
    fn stage_state(&self) -> Option<&dyn Any>;
}

impl<S: 'static> LiveDyn for Live<S> {
    fn is_open(&self) -> bool { Live::is_open(self) }
    fn region(&self) -> Region { Live::region(self) }
    fn region_at(&self, x: f32, width: f32) -> Region { Live::region_at(self, x, width) }
    fn is_shortcut(&self, ev: &KeyEvent) -> bool { Live::is_shortcut(self, ev) }
    fn side_key(&mut self, ev: &KeyEvent, tx: &mut Tx) -> bool { Live::side_key(self, ev, tx) }
    fn nav(&self, ev: &KeyEvent, region: Region) -> Nav { Live::nav(self, ev, region) }
    fn clear_filter(&mut self) -> bool { Live::clear_filter(self) }
    fn first_shown(&self) -> Option<usize> { Live::first_shown(self) }
    fn first_match(&self) -> Option<usize> { Live::first_match(self) }
    fn moved(&mut self, moved: Moved) -> bool { Live::moved(self, moved) }
    fn saw_focus(&self, focus: Option<&str>) -> bool { Live::saw_focus(self, focus) }
    fn request(&mut self, requests: Vec<Request>) { Live::request(self, requests) }
    fn has_pending(&self) -> bool { Live::has_pending(self) }
    fn pending_swaps_stage(&self) -> bool { Live::pending_swaps_stage(self) }
    fn pending_replays_only(&self) -> bool { Live::pending_replays_only(self) }
    fn polls(&self) -> bool { Live::polls(self) }
    fn poll_stage(&mut self, t: f64, tx: &mut Tx) -> bool { Live::poll_stage(self, t, tx) }
    fn take_pressed(&mut self) -> Vec<KeyEvent> { Live::take_pressed(self) }
    fn root(&self) -> usize { self.root }
    fn set_root(&mut self, root: usize) { self.root = root; }
    fn pressing(&self) -> bool { self.pressing }
    fn set_pressing(&mut self, pressing: bool) { self.pressing = pressing; }
    fn played(&self) -> u64 { self.played }
    fn app_time_scale(&self) -> f32 { self.app_time_scale }
    fn set_app_time_scale(&mut self, scale: f32) { self.app_time_scale = scale; }
    fn open_at(&mut self, story: &str, variant: Option<&str>) -> Result<(), String> { Live::open_at(self, Some(story), variant) }
    fn trigger_or_why(&self, key: char) -> Result<Request, String> { Live::trigger_or_why(self, key) }
    fn stage_state(&self) -> Option<&dyn Any> { Live::stage_state(self) }
}

/// The playground's typed half, behind a second vtable so a state type that never attaches a
/// playground builds none of its code.
pub(crate) trait LiveApp<S: 'static>: LiveDyn {
    fn live(&self) -> &Live<S>;
    fn live_mut(&mut self) -> &mut Live<S>;
    fn as_dyn(&self) -> &dyn LiveDyn;
    fn as_dyn_mut(&mut self) -> &mut dyn LiveDyn;
    fn app_states<'a>(&'a mut self, app: &'a mut S) -> States<'a>;
    fn app_states_ref<'a>(&'a self, app: &'a S) -> StatesRef<'a>;
    fn app_view(&self, app: &S, view: fn(&S, &mut Cx<S>) -> El, cx: &mut Cx<S>, viewport: (f32, f32)) -> El;
    fn app_key_state<'a>(&'a mut self, app: &'a mut S) -> Option<&'a mut S>;
    fn app_stage_hears(&self, app: &S) -> bool;
    fn app_typing(&self, app: &S) -> bool;
    fn app_apply(&mut self, clock: f64, app: &mut S) -> bool;
}

impl<S: 'static> LiveApp<S> for Live<S> {
    fn live(&self) -> &Live<S> { self }
    fn live_mut(&mut self) -> &mut Live<S> { self }
    fn as_dyn(&self) -> &dyn LiveDyn { self }
    fn as_dyn_mut(&mut self) -> &mut dyn LiveDyn { self }
    fn app_states<'a>(&'a mut self, app: &'a mut S) -> States<'a> { self.states(app) }
    fn app_states_ref<'a>(&'a self, app: &'a S) -> StatesRef<'a> { self.states_ref(app) }
    fn app_view(&self, app: &S, view: fn(&S, &mut Cx<S>) -> El, cx: &mut Cx<S>, viewport: (f32, f32)) -> El { self.view(app, view, cx, viewport) }
    fn app_key_state<'a>(&'a mut self, app: &'a mut S) -> Option<&'a mut S> { self.key_state(app) }
    fn app_stage_hears(&self, app: &S) -> bool { self.stage_hears(app) }
    fn app_typing(&self, app: &S) -> bool { self.typing(app) }
    fn app_apply(&mut self, clock: f64, app: &mut S) -> bool { self.apply(clock, app) }
}

/// The app's state, view and hooks, as a [`Runtime`] of its state type holds them.
pub struct Typed<S: 'static> {
    /// Read through [`Runtime::state`] and [`Runtime::state_mut`].
    pub(crate) state: S,
    pub(crate) view: fn(&S, &mut Cx<S>) -> El,
    /// Called at the start of every frame with the clock; return true when state changed (new data).
    pub poll: Option<fn(&mut S, f64, &mut Tx) -> bool>,
    /// Called for keys unhandled by the focused control; return true to consume it.
    pub on_key: Option<fn(&mut S, &KeyEvent, &mut Tx) -> bool>,
    /// `App::on_field_key`: keys a focused text field would get, offered first.
    pub on_field_key: Option<fn(&mut S, &KeyEvent, &mut Tx) -> bool>,
    pub(crate) on_focus: Option<fn(&mut S, bool, &mut Tx)>,
    pub(crate) on_mods: Option<fn(&mut S, crate::Mods, &mut Tx)>,
    pub(crate) on_visibility: Option<fn(&mut S, bool, &mut Tx)>,
    pub(crate) on_ime: Option<fn(&mut S, &Ime, &mut Tx) -> bool>,
    /// `App::playground`, when this run shows it.
    pub(crate) playground: Option<Box<dyn LiveApp<S>>>,
}

impl<S: 'static> Typed<S> {
    /// The state `on_key` and `on_mods` get: the stage's while the playground shows an app state.
    fn key_state(&mut self) -> Option<&mut S> {
        match &mut self.playground {
            Some(playground) => playground.app_key_state(&mut self.state),
            None => Some(&mut self.state),
        }
    }
}

impl<S: 'static> AppSide for Typed<S> {
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn states(&mut self) -> States<'_> {
        match &mut self.playground {
            Some(playground) => playground.app_states(&mut self.state),
            None => States::one(handler_state(&mut self.state)),
        }
    }

    fn states_ref(&self) -> StatesRef<'_> {
        match &self.playground {
            Some(playground) => playground.app_states_ref(&self.state),
            None => StatesRef::one(handler_state_ref(&self.state)),
        }
    }

    fn root_view(&self, cx: CxAny, viewport: (f32, f32)) -> (El, CxAny) {
        let mut cx = cx.into_state::<S>();
        let root = match &self.playground {
            Some(playground) => playground.app_view(&self.state, self.view, &mut cx, viewport),
            None => crate::playground::call(self.view, &self.state, &mut cx),
        };
        (root, cx.into_state())
    }

    fn has(&self, hook: Hook) -> bool {
        match hook {
            Hook::Poll => self.poll.is_some(),
            Hook::Key => self.on_key.is_some(),
            Hook::FieldKey => self.on_field_key.is_some(),
            Hook::Focus => self.on_focus.is_some(),
            Hook::Mods => self.on_mods.is_some(),
            Hook::Visibility => self.on_visibility.is_some(),
            Hook::Ime => self.on_ime.is_some(),
        }
    }

    fn poll(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let Some(poll) = self.poll else { return false };
        app_call!(poll, &mut self.state, clock, tx)
    }

    fn key(&mut self, field: bool, ev: &KeyEvent, tx: &mut Tx) -> bool {
        let Some(f) = (if field { self.on_field_key } else { self.on_key }) else { return false };
        self.key_state().is_some_and(|state| app_call!(f, state, ev, tx))
    }

    fn focus(&mut self, focused: bool, tx: &mut Tx) {
        if let Some(f) = self.on_focus { f(&mut self.state, focused, tx); }
    }

    fn visibility(&mut self, visible: bool, tx: &mut Tx) {
        if let Some(f) = self.on_visibility { f(&mut self.state, visible, tx); }
    }

    fn mods(&mut self, mods: crate::Mods, tx: &mut Tx) -> bool {
        let Some(f) = self.on_mods else { return false };
        let Some(state) = self.key_state() else { return false };
        f(state, mods, tx);
        true
    }

    fn ime(&mut self, ev: &Ime, tx: &mut Tx) -> bool {
        let Some(f) = self.on_ime else { return false };
        let state = match &mut self.playground {
            Some(playground) if !playground.app_stage_hears(&self.state) => None,
            Some(playground) => playground.app_key_state(&mut self.state),
            None => Some(&mut self.state),
        };
        state.is_some_and(|state| Tx::took(app_call!(f, state, ev, tx), tx))
    }

    fn playground(&self) -> Option<&dyn LiveDyn> { self.playground.as_deref().map(|p| p.as_dyn()) }
    fn playground_mut(&mut self) -> Option<&mut dyn LiveDyn> { self.playground.as_deref_mut().map(|p| p.as_dyn_mut()) }
    fn playground_typing(&self) -> bool { self.playground.as_ref().is_some_and(|p| p.app_typing(&self.state)) }
    fn playground_apply(&mut self, clock: f64) -> bool {
        let Typed { playground, state, .. } = self;
        playground.as_mut().is_some_and(|p| p.app_apply(clock, state))
    }
}

/// A window's runtime for an app whose state is `S`. Everything that doesn't name `S` is
/// [`RuntimeCore`]'s, which this derefs to.
pub struct Runtime<S: 'static> {
    core: RuntimeCore,
    _s: PhantomData<fn(&S)>,
}

impl<S: 'static> Deref for Runtime<S> {
    type Target = RuntimeCore;
    fn deref(&self) -> &RuntimeCore { &self.core }
}

impl<S: 'static> DerefMut for Runtime<S> {
    fn deref_mut(&mut self) -> &mut RuntimeCore { &mut self.core }
}

impl<S: 'static> Runtime<S> {
    pub fn new(state: S, view: fn(&S, &mut Cx<S>) -> El, viewport: (f32, f32), env: Env) -> Self {
        Self::with_text(state, view, viewport, env, TextSystem::default())
    }

    pub fn with_text(state: S, view: fn(&S, &mut Cx<S>) -> El, viewport: (f32, f32), env: Env, text: TextSystem) -> Self {
        let app = Typed { state, view, poll: None, on_key: None, on_field_key: None, on_focus: None, on_mods: None, on_visibility: None, on_ime: None, playground: None };
        Runtime { core: RuntimeCore::with_app(Box::new(app), viewport, env, text), _s: PhantomData }
    }

    /// The app's state, view and hooks.
    pub fn app(&self) -> &Typed<S> {
        self.core.app.as_any().downcast_ref().expect("a Runtime<S> holds a Typed<S>")
    }

    pub fn app_mut(&mut self) -> &mut Typed<S> {
        self.core.app.as_any_mut().downcast_mut().expect("a Runtime<S> holds a Typed<S>")
    }

    pub fn state(&self) -> &S { &self.app().state }
    pub fn state_mut(&mut self) -> &mut S { &mut self.app_mut().state }

    /// Changes the state from outside any handler, for data another thread delivered
    /// (`AppHandle::update`). Runs now, so it works while no window shows; the next frame commits
    /// it with the motion `f` asked for.
    pub fn update(&mut self, f: impl FnOnce(&mut S, &mut Tx)) {
        self.update_if(|s, tx| { f(s, tx); true });
    }

    /// `update` for a change that may be no change: `f` returns whether it changed state the
    /// view reads, as `poll` does.
    pub fn update_if(&mut self, f: impl FnOnce(&mut S, &mut Tx) -> bool) -> bool {
        let mut f = Some(f);
        self.core.update_app(&mut |app, tx| {
            let app: &mut Typed<S> = app.as_any_mut().downcast_mut().expect("a Runtime<S> holds a Typed<S>");
            f.take().is_some_and(|f| f(&mut app.state, tx))
        })
    }
}
