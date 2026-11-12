// The state of each step of a running search, built from the events in
// `pipeline.rs`. The Run / Info tab draws it as a card and the run bar shows
// the current step. Written by Benjamin A. Neely (NIST) on 2026-11-12.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::pipeline::{Event, Step};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Pending,
    Running,
    Done,
    Skipped,
    /// The run failed or was stopped while this step ran.
    Stopped,
}

#[derive(Clone, Debug)]
pub struct StepState {
    pub step: Step,
    pub status: Status,
    /// The right-hand text: what the step is doing, or its summary when done.
    pub detail: String,
    /// `done` of `total` units, when the step has a count.
    pub done: usize,
    pub total: Option<usize>,
    pub started: Option<Instant>,
    pub elapsed: Option<Duration>,
}

impl StepState {
    fn new(step: Step) -> Self {
        Self {
            step,
            status: Status::Pending,
            detail: String::new(),
            done: 0,
            total: None,
            started: None,
            elapsed: None,
        }
    }

    /// The bar's fill, 0 to 1. `None` means a spinner: running with no count.
    pub fn fraction(&self) -> Option<f32> {
        match self.status {
            Status::Pending => Some(0.0),
            Status::Done | Status::Skipped => Some(1.0),
            Status::Running | Status::Stopped => match self.total {
                Some(t) if t > 0 => Some((self.done as f32 / t as f32).min(1.0)),
                _ if self.status == Status::Stopped => Some(0.0),
                _ => None,
            },
        }
    }
}

/// Every step of one search.
pub struct RunProgress {
    pub steps: Vec<StepState>,
    /// The live count of spectra searched, shared with the search thread.
    pub searched: Arc<AtomicUsize>,
    pub started: Instant,
    pub finished: Option<Duration>,
}

impl RunProgress {
    pub fn new(searched: Arc<AtomicUsize>) -> Self {
        Self {
            steps: Step::ALL.iter().map(|s| StepState::new(*s)).collect(),
            searched,
            started: Instant::now(),
            finished: None,
        }
    }

    fn get(&mut self, step: Step) -> &mut StepState {
        self.steps
            .iter_mut()
            .find(|s| s.step == step)
            .expect("every step has a row")
    }

    pub fn apply(&mut self, event: Event) {
        match event {
            Event::Started(step) => {
                let s = self.get(step);
                s.status = Status::Running;
                s.started.get_or_insert_with(Instant::now);
            }
            Event::Detail(step, text) => self.get(step).detail = text,
            Event::Counter(step, done, total) => {
                let s = self.get(step);
                s.done = done;
                s.total = Some(total);
            }
            Event::Finished(step, text) => {
                let s = self.get(step);
                s.status = Status::Done;
                s.detail = text;
                s.elapsed = s.started.map(|t| t.elapsed());
                if let Some(t) = s.total {
                    s.done = t;
                }
            }
            Event::Skipped(step, text) => {
                let s = self.get(step);
                s.status = Status::Skipped;
                s.detail = text;
            }
        }
    }

    /// Reads the live search counter into the Search row. Call each frame.
    pub fn poll(&mut self) {
        let searched = self.searched.load(Ordering::Relaxed);
        let s = self.get(Step::Search);
        if s.status == Status::Running {
            s.done = searched;
        }
    }

    /// The run ended. A step still running is marked stopped, with `why`.
    pub fn end(&mut self, why: Option<&str>) {
        self.finished = Some(self.started.elapsed());
        for s in &mut self.steps {
            if s.status == Status::Running {
                s.status = Status::Stopped;
                if let Some(why) = why {
                    s.detail = why.to_string();
                }
            }
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    /// The step to show in the run bar: the last one that started and is
    /// still running.
    pub fn current(&self) -> Option<&StepState> {
        self.steps
            .iter()
            .rev()
            .find(|s| s.status == Status::Running)
    }

    pub fn elapsed(&self) -> Duration {
        self.finished.unwrap_or_else(|| self.started.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progress() -> RunProgress {
        RunProgress::new(Arc::new(AtomicUsize::new(0)))
    }

    #[test]
    fn a_step_runs_then_finishes() {
        let mut p = progress();
        assert_eq!(p.steps.len(), Step::ALL.len());
        p.apply(Event::Started(Step::Digest));
        assert_eq!(p.current().unwrap().step, Step::Digest);
        assert_eq!(p.current().unwrap().fraction(), None, "no count: a spinner");
        p.apply(Event::Finished(Step::Digest, "1,000 peptides".into()));
        assert!(p.current().is_none());
        let d = &p.steps[1];
        assert_eq!(d.status, Status::Done);
        assert_eq!(d.detail, "1,000 peptides");
        assert_eq!(d.fraction(), Some(1.0));
    }

    #[test]
    fn the_search_bar_follows_the_live_counter() {
        let mut p = progress();
        p.apply(Event::Started(Step::Search));
        p.apply(Event::Counter(Step::Search, 0, 200));
        p.searched.store(50, Ordering::Relaxed);
        p.poll();
        assert_eq!(p.current().unwrap().fraction(), Some(0.25));
        p.searched.store(200, Ordering::Relaxed);
        p.poll();
        assert_eq!(p.current().unwrap().fraction(), Some(1.0));
    }

    #[test]
    fn a_stopped_run_marks_the_running_step() {
        let mut p = progress();
        p.apply(Event::Started(Step::ReadSpectra));
        p.end(Some("stopped"));
        assert!(p.is_finished());
        let r = &p.steps[3];
        assert_eq!(r.status, Status::Stopped);
        assert_eq!(r.detail, "stopped");
        assert_eq!(
            p.steps[4].status,
            Status::Pending,
            "later steps stay pending"
        );
    }
}
