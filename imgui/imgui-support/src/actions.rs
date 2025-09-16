//
// gui: actions
//

// ----------------------------------------------------------------------------
#[macro_export]
macro_rules! ima_seq {
    ($($action:expr), *) => {
        {
            let mut actions = Vec::new();
            $(
                actions.push($action.into());
            )*
            ::imgui_support::actions::Sequence::new(false, actions)
        }
    }
}
// ----------------------------------------------------------------------------
#[macro_export]
macro_rules! ima_prio_seq {
    ($($action:expr), *) => {
        {
            let mut actions = Vec::new();
            $(
                actions.push($action.into());
            )*
            ::imgui_support::actions::Sequence::new(true, actions)
        }
    }
}
// ----------------------------------------------------------------------------
// external interface
// ----------------------------------------------------------------------------
pub struct ActionState<A> {
    queue: Vec<A>,

    confirm: Option<String>,
    yes: Vec<A>,
    no: Vec<A>,
}
// ----------------------------------------------------------------------------
#[derive(Debug)]
pub struct Sequence<A> {
    actions: Vec<A>,
    prio: bool,
}
// ----------------------------------------------------------------------------
pub fn perform<A>(ui: &Ui<'_>, actions: &mut ActionState<A>) -> Option<A> {
    if actions.confirmation_pending() {
        if let Some(result) = windows::request_yes_no(ui, actions.confirmation().unwrap()) {
            match result {
                Confirm::Yes => actions.resolve_confirmation(true),
                Confirm::No => actions.resolve_confirmation(false),
                Confirm::Cancel => actions.clear(),
            }
        }
        None
    } else {
        actions.next()
    }
}
// ----------------------------------------------------------------------------
// internals
// ----------------------------------------------------------------------------
use imgui::Ui;
use windows;
use windows::Confirm;
// ----------------------------------------------------------------------------
impl<A> Sequence<A> {
    // ------------------------------------------------------------------------
    pub fn new(prio: bool, actions: Vec<A>) -> Sequence<A> {
        Sequence { prio, actions }
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
impl<A> Default for ActionState<A> {
    fn default() -> ActionState<A> {
        ActionState {
            queue: Vec::default(),
            confirm: None,
            yes: Vec::default(),
            no: Vec::default(),
        }
    }
}
// ----------------------------------------------------------------------------
impl<A> ActionState<A> {
    // ------------------------------------------------------------------------
    #[inline]
    pub fn filter_push<B: Into<A>>(&mut self, action: Option<B>) {
        if let Some(action) = action {
            self.queue.push(action.into());
        }
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn filter_include(&mut self, sequence: Option<Sequence<A>>) {
        if let Some(seq) = sequence {
            self.include(seq);
        }
    }
    // ------------------------------------------------------------------------
    pub fn push<B: Into<A>>(&mut self, action: B) {
        self.queue.push(action.into());
    }
    // ------------------------------------------------------------------------
    pub fn include(&mut self, mut sequence: Sequence<A>) {
        if sequence.prio {
            // prepend whole sequence
            let new_queue = sequence.actions
                .drain(..)
                // .map(|a| a.into())
                .chain(self.queue.drain(..))
                .collect();

            self.queue = new_queue;
        } else {
            for action in sequence.actions.drain(..) {
                self.queue.push(action);
            }
        }
    }
    // ------------------------------------------------------------------------
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<A> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }
    // ------------------------------------------------------------------------
    pub fn clear(&mut self) {
        self.queue.clear();
        self.yes.clear();
        self.no.clear();
        self.confirm = None;
    }
    // ------------------------------------------------------------------------
    pub fn set_interactive(&mut self, text: String, yes: Vec<A>, no: Vec<A>) {
        self.confirm = Some(text);
        self.yes = yes;
        self.no = no;
    }
    // ------------------------------------------------------------------------
    #[inline]
    pub fn confirmation_pending(&self) -> bool {
        self.confirm.is_some()
    }
    // ------------------------------------------------------------------------
    pub fn confirmation(&self) -> Option<&String> {
        self.confirm.as_ref()
    }
    // ------------------------------------------------------------------------
    pub fn resolve_confirmation(&mut self, result: bool) {
        if result {
            self.yes.append(&mut self.queue);
            self.queue.append(&mut self.yes);
        } else {
            self.no.append(&mut self.queue);
            self.queue.append(&mut self.no);
        }
        self.confirm = None;
        self.yes = Vec::new();
        self.no = Vec::new();
    }
    // ------------------------------------------------------------------------
}
// ----------------------------------------------------------------------------
