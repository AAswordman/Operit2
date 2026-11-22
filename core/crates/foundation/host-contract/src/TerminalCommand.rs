//! Cooperative command observation, with finite OS operations kept on the host blocking boundary.

use crate::{HostError, HostResult, HostRuntimeTaskSchedulerHost, TerminalCommandOutput};
use std::collections::VecDeque;
use std::future::poll_fn;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// Advances one session-owned command without waiting for additional terminal bytes.
pub trait TerminalCommandExecution: Send {
    /// Drains available output and returns a receipt only after the command finishes.
    fn pollOutput(
        &mut self,
        context: &mut Context<'_>,
        expired: bool,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> Poll<HostResult<TerminalCommandOutput>>;

    /// Reports the command lifetime; zero retains session-owned execution until completion or close.
    fn timeoutMs(&self) -> u64;

    /// Sends an interrupt without waiting for the shell to acknowledge it.
    fn beginInterrupt(&mut self) -> HostResult<()>;

    /// Registers an output wakeup until the interrupted shell acknowledges readiness.
    fn pollInterrupt(&mut self, context: &mut Context<'_>, expired: bool) -> Poll<HostResult<()>>;

    /// Releases session ownership after completion or acknowledged interruption.
    fn finish(
        self: Box<Self>,
        result: HostResult<TerminalCommandOutput>,
        settled: HostResult<()>,
    ) -> HostResult<TerminalCommandOutput>;
}

/// Preserves the owning backend state while adapting the public completion metadata.
pub fn mapTerminalCommandExecution(
    execution: Box<dyn TerminalCommandExecution>,
    map: impl FnOnce(TerminalCommandOutput) -> HostResult<TerminalCommandOutput> + Send + 'static,
) -> Box<dyn TerminalCommandExecution> {
    Box::new(MappedCommand { execution, map })
}

/// Delegates progress and cancellation to the backend that owns the command.
struct MappedCommand<F> {
    execution: Box<dyn TerminalCommandExecution>,
    map: F,
}

impl<F> TerminalCommandExecution for MappedCommand<F>
where
    F: FnOnce(TerminalCommandOutput) -> HostResult<TerminalCommandOutput> + Send,
{
    /// Polls the original backend's producer events.
    fn pollOutput(
        &mut self,
        context: &mut Context<'_>,
        expired: bool,
        onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
    ) -> Poll<HostResult<TerminalCommandOutput>> {
        self.execution.pollOutput(context, expired, onOutput)
    }
    /// Retains the original execution deadline.
    fn timeoutMs(&self) -> u64 {
        self.execution.timeoutMs()
    }
    /// Interrupts the exact backend that submitted the command.
    fn beginInterrupt(&mut self) -> HostResult<()> {
        self.execution.beginInterrupt()
    }
    /// Waits for the original backend to acknowledge interruption.
    fn pollInterrupt(&mut self, context: &mut Context<'_>, expired: bool) -> Poll<HostResult<()>> {
        self.execution.pollInterrupt(context, expired)
    }
    /// Applies metadata conversion only after backend ownership has been released.
    fn finish(
        self: Box<Self>,
        result: HostResult<TerminalCommandOutput>,
        settled: HostResult<()>,
    ) -> HostResult<TerminalCommandOutput> {
        self.execution.finish(result, settled).and_then(self.map)
    }
}

/// Retains a blocking operation's result and the current asynchronous consumer.
struct BlockingCompletion<T> {
    result: Option<T>,
    waker: Option<Waker>,
}

/// Awaits finite host work without occupying the caller's asynchronous executor thread.
pub async fn runTerminalBlocking<T: Send + 'static>(
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    name: &'static str,
    work: impl FnOnce() -> HostResult<T> + Send + 'static,
) -> HostResult<T> {
    let state = Arc::new(Mutex::new(BlockingCompletion {
        result: None,
        waker: None,
    }));
    let completed = state.clone();
    scheduler.scheduleHostRuntimeTask(
        name,
        Box::new(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .unwrap_or_else(|_| Err(HostError::new("Terminal host operation panicked")));
            let waker = {
                let mut state = completed.lock().unwrap_or_else(|error| error.into_inner());
                state.result = Some(result);
                state.waker.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        }),
    )?;
    poll_fn(move |context| {
        let mut state = state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(result) = state.result.take() {
            return Poll::Ready(result);
        }
        state.waker = Some(context.waker().clone());
        Poll::Pending
    })
    .await
}

/// Polls command output cooperatively; execution ownership survives the caller's foreground wait.
pub async fn observeTerminalCommand(
    mut execution: Box<dyn TerminalCommandExecution>,
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    mut onOutput: crate::TerminalOutputCallback,
) -> HostResult<TerminalCommandOutput> {
    let result = waitForCommandEvent(
        scheduler.clone(),
        execution.timeoutMs(),
        |context, expired| {
            execution.pollOutput(context, expired, &mut |bytes| onOutput(bytes, false))
        },
    )
    .await
    .and_then(|receipt| onOutput(&[], true).map(|_| receipt));
    let settled = if needsInterrupt(&result) {
        let (returned, started) =
            runTerminalBlocking(scheduler.clone(), "terminal-command-interrupt", move || {
                let started = execution.beginInterrupt();
                Ok((execution, started))
            })
            .await?;
        execution = returned;
        match started {
            Ok(()) => {
                waitForCommandEvent(scheduler.clone(), 3000, |context, expired| {
                    execution.pollInterrupt(context, expired)
                })
                .await
            }
            Err(error) => Err(error),
        }
    } else {
        Ok(())
    };
    runTerminalBlocking(scheduler, "terminal-command-finish", move || {
        execution.finish(result, settled)
    })
    .await
}

/// Waits only for a producer wakeup or a single absolute deadline.
async fn waitForCommandEvent<T>(
    scheduler: Arc<dyn HostRuntimeTaskSchedulerHost>,
    timeoutMs: u64,
    mut poll: impl FnMut(&mut Context<'_>, bool) -> Poll<HostResult<T>>,
) -> HostResult<T> {
    let mut deadline = (timeoutMs != 0).then(|| scheduler.waitForHostRuntimeDelay(timeoutMs));
    let mut expired = false;
    poll_fn(|context| {
        if let Some(timer) = deadline.as_mut() {
            match timer.as_mut().poll(context) {
                Poll::Ready(Ok(())) => {
                    expired = true;
                    deadline = None;
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                Poll::Pending => {}
            }
        }
        poll(context, expired)
    })
    .await
}

/// Requires interruption when observation failed or the command exceeded its execution deadline.
fn needsInterrupt(result: &HostResult<TerminalCommandOutput>) -> bool {
    result.as_ref().map_or(true, |receipt| receipt.timedOut)
}

/// Retains terminal bytes and terminal failures independently of a consumer's waiting strategy.
pub struct TerminalCommandBuffer {
    state: Mutex<CommandBufferState>,
    limit: usize,
}

/// Registers output and its wakeup atomically so fragmented output cannot lose notifications.
#[derive(Default)]
struct CommandBufferState {
    bytes: VecDeque<u8>,
    error: Option<HostError>,
    waker: Option<Waker>,
}

impl TerminalCommandBuffer {
    /// Creates a bounded session-owned buffer.
    pub fn new(limit: usize) -> Self {
        Self {
            state: Mutex::new(CommandBufferState::default()),
            limit,
        }
    }

    /// Publishes bytes before waking either asynchronous or explicit synchronous consumers.
    pub fn append(&self, bytes: &[u8]) {
        let waker = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.bytes.extend(bytes.iter().copied());
            while state.bytes.len() > self.limit {
                state.bytes.pop_front();
            }
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Makes close and reader failures observable even while the command produces no output.
    pub fn fail(&self, error: HostError) {
        let waker = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            state.error.get_or_insert(error);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Discards consumed command bytes while retaining a permanent reader failure.
    pub fn clear(&self) -> HostResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::new("Terminal output state poisoned"))?;
        if let Some(error) = &state.error {
            return Err(HostError::new(error.message.clone()));
        }
        state.bytes.clear();
        state.waker = None;
        Ok(())
    }

    /// Drains available bytes and installs the wakeup for subsequent output under the same lock.
    pub fn pollDrain(
        &self,
        context: &Context<'_>,
        collected: &mut Vec<u8>,
    ) -> HostResult<Option<HostError>> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| HostError::new("Terminal output state poisoned"))?;
        collected.extend(state.bytes.drain(..));
        state.waker = Some(context.waker().clone());
        Ok(state
            .error
            .as_ref()
            .map(|error| HostError::new(error.message.clone())))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HostRuntimeAsyncTask, HostRuntimeTask, HostRuntimeTurnFuture};
    use std::future::Future;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::task::Wake;

    #[derive(Default)]
    struct EventWake(AtomicUsize);
    impl Wake for EventWake {
        /// Counts actual producer wakeups without running a timer or executor thread.
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[derive(Default)]
    struct DeadlineState {
        expired: AtomicBool,
        waker: Mutex<Option<Waker>>,
    }
    impl DeadlineState {
        /// Resolves exactly one controlled deadline and wakes its registered observer.
        fn expire(&self) {
            self.expired.store(true, Ordering::SeqCst);
            if let Some(waker) = self.waker.lock().unwrap().take() {
                waker.wake();
            }
        }
    }

    #[derive(Default)]
    struct Scheduler {
        blocking: AtomicUsize,
        deadlines: Mutex<Vec<Arc<DeadlineState>>>,
    }
    impl HostRuntimeTaskSchedulerHost for Scheduler {
        /// Supplies a stable test clock; deadlines are explicitly triggered by each test.
        fn monotonicTimeMillis(&self) -> HostResult<u64> {
            Ok(0)
        }
        /// Executes finite operations inline so an accidental output wait would stall the test.
        fn scheduleHostRuntimeTask(&self, _: &str, work: HostRuntimeTask) -> HostResult<()> {
            self.blocking.fetch_add(1, Ordering::SeqCst);
            work();
            Ok(())
        }
        /// Rejects unexpected background execution in manually polled observer tests.
        fn scheduleHostRuntimeAsyncTask(&self, _: &str, _: HostRuntimeAsyncTask) -> HostResult<()> {
            panic!("unexpected task")
        }
        /// Rejects unexpected background execution in manually polled observer tests.
        fn scheduleHostRuntimeCooperativeAsyncTask(
            &self,
            _: &str,
            _: HostRuntimeAsyncTask,
        ) -> HostResult<()> {
            panic!("unexpected task")
        }
        /// Ensures observation never substitutes delayed work for output events.
        fn scheduleDelayedHostRuntimeTask(
            &self,
            _: &str,
            _: u64,
            _: HostRuntimeTask,
        ) -> HostResult<()> {
            panic!("unexpected delayed task")
        }
        /// Registers a single manually controlled absolute deadline.
        fn waitForHostRuntimeDelay(&self, _: u64) -> HostRuntimeTurnFuture {
            let deadline = Arc::new(DeadlineState::default());
            self.deadlines.lock().unwrap().push(deadline.clone());
            Box::pin(poll_fn(move |context| {
                let mut waker = deadline.waker.lock().unwrap();
                if deadline.expired.load(Ordering::SeqCst) {
                    return Poll::Ready(Ok(()));
                }
                *waker = Some(context.waker().clone());
                Poll::Pending
            }))
        }
    }

    struct Command {
        output: Arc<TerminalCommandBuffer>,
        timeout: u64,
        interrupted: Arc<AtomicBool>,
        finished: Arc<AtomicBool>,
        collected: Vec<u8>,
    }
    impl TerminalCommandExecution for Command {
        /// Models a fragmented completion record and preserves failure notifications.
        fn pollOutput(
            &mut self,
            context: &mut Context<'_>,
            expired: bool,
            onOutput: &mut dyn FnMut(&[u8]) -> HostResult<()>,
        ) -> Poll<HostResult<TerminalCommandOutput>> {
            let offset = self.collected.len();
            let error = self.output.pollDrain(context, &mut self.collected).unwrap();
            if let Err(error) = onOutput(&self.collected[offset..]) {
                return Poll::Ready(Err(error));
            }
            if self.collected.ends_with(b"done") || expired {
                return Poll::Ready(Ok(TerminalCommandOutput {
                    command: "test".into(),
                    sessionId: "session".into(),
                    platform: "test".into(),
                    terminal: "test".into(),
                    terminalType: "shell".into(),
                    output: String::from_utf8_lossy(&self.collected).into(),
                    exitCode: if expired { -1 } else { 0 },
                    timedOut: expired,
                }));
            }
            match error {
                Some(error) => Poll::Ready(Err(error)),
                None => Poll::Pending,
            }
        }
        /// Supplies the test execution budget independently from any foreground observer.
        fn timeoutMs(&self) -> u64 {
            self.timeout
        }
        /// Marks submission without waiting for an acknowledgement.
        fn beginInterrupt(&mut self) -> HostResult<()> {
            self.interrupted.store(true, Ordering::SeqCst);
            self.collected.clear();
            Ok(())
        }
        /// Requires an actual cancellation acknowledgement or a close event.
        fn pollInterrupt(
            &mut self,
            context: &mut Context<'_>,
            expired: bool,
        ) -> Poll<HostResult<()>> {
            let error = self.output.pollDrain(context, &mut self.collected).unwrap();
            if self.collected == b"ready" {
                return Poll::Ready(Ok(()));
            }
            if let Some(error) = error {
                return Poll::Ready(Err(error));
            }
            if expired {
                return Poll::Ready(Err(HostError::new("interrupt expired")));
            }
            Poll::Pending
        }
        /// Publishes release only after the same command state settles.
        fn finish(
            self: Box<Self>,
            result: HostResult<TerminalCommandOutput>,
            settled: HostResult<()>,
        ) -> HostResult<TerminalCommandOutput> {
            self.finished.store(true, Ordering::SeqCst);
            settled.and(result)
        }
    }

    /// Constructs observable command state without using a real shell or blocking reader.
    fn command(timeout: u64) -> Command {
        Command {
            output: Arc::new(TerminalCommandBuffer::new(1024)),
            timeout,
            interrupted: Arc::default(),
            finished: Arc::default(),
            collected: Vec::new(),
        }
    }

    /// More pending commands than blocking workers must suspend without timers or worker work.
    #[test]
    fn silent_commands_do_not_reserve_workers_or_polling_timers() {
        let scheduler = Arc::new(Scheduler::default());
        let wake = Arc::new(EventWake::default());
        let waker = Waker::from(wake.clone());
        let mut context = Context::from_waker(&waker);
        let mut commands = Vec::new();
        for _ in 0..32 {
            let command = command(0);
            let output = command.output.clone();
            let mut observation = Box::pin(observeTerminalCommand(
                Box::new(command),
                scheduler.clone(),
                Box::new(|_, _| Ok(())),
            ));
            assert!(observation.as_mut().poll(&mut context).is_pending());
            commands.push((output, observation));
        }
        assert_eq!(scheduler.blocking.load(Ordering::SeqCst), 0);
        assert!(scheduler.deadlines.lock().unwrap().is_empty());
        assert_eq!(wake.0.load(Ordering::SeqCst), 0);
        for (output, mut observation) in commands {
            output.append(b"do");
            assert!(observation.as_mut().poll(&mut context).is_pending());
            output.append(b"ne");
            assert!(matches!(
                observation.as_mut().poll(&mut context),
                Poll::Ready(Ok(_))
            ));
        }
        assert_eq!(
            scheduler.blocking.load(Ordering::SeqCst),
            32,
            "only finite finalization may use a worker"
        );
        assert!(scheduler.deadlines.lock().unwrap().is_empty());
    }

    /// Execution timeout must suspend through cancellation and retain busy ownership until readiness.
    #[test]
    fn timeout_waits_for_interrupt_event_without_a_worker_or_periodic_timer() {
        let scheduler = Arc::new(Scheduler::default());
        let command = command(50);
        let output = command.output.clone();
        let interrupted = command.interrupted.clone();
        let finished = command.finished.clone();
        let waker = Waker::from(Arc::new(EventWake::default()));
        let mut context = Context::from_waker(&waker);
        let mut observation = Box::pin(observeTerminalCommand(
            Box::new(command),
            scheduler.clone(),
            Box::new(|_, _| Ok(())),
        ));
        assert!(observation.as_mut().poll(&mut context).is_pending());
        scheduler.deadlines.lock().unwrap()[0].expire();
        assert!(observation.as_mut().poll(&mut context).is_pending());
        assert!(interrupted.load(Ordering::SeqCst));
        assert!(!finished.load(Ordering::SeqCst));
        assert_eq!(scheduler.blocking.load(Ordering::SeqCst), 1);
        assert_eq!(
            scheduler.deadlines.lock().unwrap().len(),
            2,
            "one execution and one cancellation deadline"
        );
        output.append(b"ready");
        let Poll::Ready(Ok(receipt)) = observation.as_mut().poll(&mut context) else {
            panic!("missing receipt")
        };
        assert!(receipt.timedOut);
        assert!(finished.load(Ordering::SeqCst));
    }

    /// A silent EOF must wake the observer and release execution without another output frame.
    #[test]
    fn close_wakes_a_silent_command_and_final_marker_precedes_eof() {
        for finalOutput in [false, true] {
            let scheduler = Arc::new(Scheduler::default());
            let command = command(0);
            let output = command.output.clone();
            let finished = command.finished.clone();
            let wake = Arc::new(EventWake::default());
            let waker = Waker::from(wake.clone());
            let mut context = Context::from_waker(&waker);
            let mut observation = Box::pin(observeTerminalCommand(
                Box::new(command),
                scheduler,
                Box::new(|_, _| Ok(())),
            ));
            assert!(observation.as_mut().poll(&mut context).is_pending());
            if finalOutput {
                output.append(b"done");
            }
            output.fail(HostError::new("closed"));
            assert!(wake.0.load(Ordering::SeqCst) > 0);
            let Poll::Ready(result) = observation.as_mut().poll(&mut context) else {
                panic!("silent close did not settle")
            };
            assert_eq!(result.is_ok(), finalOutput);
            assert!(finished.load(Ordering::SeqCst));
        }
    }
}
