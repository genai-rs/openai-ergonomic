//! Running the tool calls of a turn: concurrently where the tools allow it, answered in call
//! order.
//!
//! The gate is a FIFO read/write lock without a lock: a call of a tool that
//! [`supports_parallel`](crate::FunctionTool::supports_parallel) starts while only such calls
//! run; any other call starts once everything before it finished, and everything after it waits
//! for it. The futures run on the loop's own task (no spawning), so nothing needs to be `Send` on
//! WebAssembly.

use std::collections::VecDeque;

use futures::stream::{FuturesUnordered, StreamExt};

use crate::time::Instant;
use crate::tool_framework::{CallAnswer, ToolError, ToolInvocation, ToolRegistry};
use crate::BoxFuture;

type Running<'r> = BoxFuture<'r, (usize, Result<CallAnswer, ToolError>)>;

struct Slot {
    call: ToolInvocation,
    parallel: bool,
    started: Option<Instant>,
    answer: Option<CallAnswer>,
}

pub(super) struct Dispatcher<'r> {
    registry: &'r ToolRegistry,
    slots: Vec<Slot>,
    queue: VecDeque<usize>,
    running: FuturesUnordered<Running<'r>>,
    parallel_running: usize,
    exclusive_running: bool,
}

impl<'r> Dispatcher<'r> {
    pub(super) fn new(registry: &'r ToolRegistry) -> Self {
        Self {
            registry,
            slots: Vec::new(),
            queue: VecDeque::new(),
            running: FuturesUnordered::new(),
            parallel_running: 0,
            exclusive_running: false,
        }
    }

    /// Queue a call; it starts as soon as the gate lets it.
    pub(super) fn push(&mut self, call: ToolInvocation, parallel: bool) {
        self.slots.push(Slot {
            call,
            parallel,
            started: None,
            answer: None,
        });
        self.queue.push_back(self.slots.len() - 1);
        self.start_ready();
    }

    /// Add a call answered already (such as the loop's own `tool_search`).
    pub(super) fn push_answered(&mut self, call: ToolInvocation, answer: CallAnswer) {
        self.slots.push(Slot {
            call,
            parallel: true,
            started: Some(Instant::now()),
            answer: Some(answer),
        });
    }

    fn start_ready(&mut self) {
        while let Some(&index) = self.queue.front() {
            let parallel = self.slots[index].parallel;
            let may_start = if parallel {
                !self.exclusive_running
            } else {
                !self.exclusive_running && self.parallel_running == 0
            };
            if !may_start {
                return;
            }
            self.queue.pop_front();
            if parallel {
                self.parallel_running += 1;
            } else {
                self.exclusive_running = true;
            }
            let slot = &mut self.slots[index];
            slot.started = Some(Instant::now());
            let call = slot.call.clone();
            let registry = self.registry;
            self.running.push(Box::pin(async move {
                (index, registry.answer_call(call).await)
            }));
        }
    }

    /// Whether every call was answered.
    pub(super) fn is_idle(&self) -> bool {
        self.queue.is_empty() && self.running.is_empty()
    }

    /// Wait for the next call to finish and return its slot; a fatal failure is returned as the
    /// error. Pending forever while no call runs.
    pub(super) async fn finished(&mut self) -> Result<usize, ToolError> {
        let Some((index, result)) = self.running.next().await else {
            return futures::future::pending().await;
        };
        if self.slots[index].parallel {
            self.parallel_running -= 1;
        } else {
            self.exclusive_running = false;
        }
        let answer = result?;
        self.slots[index].answer = Some(answer);
        self.start_ready();
        Ok(index)
    }

    /// The answer of a finished slot.
    pub(super) fn answer(&self, index: usize) -> Option<&CallAnswer> {
        self.slots.get(index)?.answer.as_ref()
    }

    /// The calls, in order, with their answers so far; the dispatcher is emptied.
    pub(super) fn take(&mut self) -> Vec<(ToolInvocation, Option<CallAnswer>)> {
        self.queue.clear();
        self.running = FuturesUnordered::new();
        self.parallel_running = 0;
        self.exclusive_running = false;
        std::mem::take(&mut self.slots)
            .into_iter()
            .map(|slot| (slot.call, slot.answer))
            .collect()
    }

    /// Stop every call: the answers of the finished calls, `"aborted by user after …s"` for the
    /// running ones and `"aborted"` for the others, in call order. The dispatcher is emptied.
    pub(super) fn abort(&mut self) -> Vec<CallAnswer> {
        let started: Vec<Option<Instant>> = self.slots.iter().map(|slot| slot.started).collect();
        self.take()
            .into_iter()
            .zip(started)
            .map(|((call, answer), started)| match (answer, started) {
                (Some(answer), _) => answer,
                (None, Some(started)) => CallAnswer::failure(
                    &call,
                    format!(
                        "aborted by user after {:.1}s",
                        started.elapsed().as_secs_f64()
                    ),
                ),
                (None, None) => CallAnswer::aborted(&call),
            })
            .collect()
    }

    /// The finished answers, in call order, of a dispatcher stopped by a fatal failure. The
    /// dispatcher is emptied.
    pub(super) fn finished_answers(&mut self) -> Vec<CallAnswer> {
        self.take()
            .into_iter()
            .filter_map(|(_, answer)| answer)
            .collect()
    }
}
