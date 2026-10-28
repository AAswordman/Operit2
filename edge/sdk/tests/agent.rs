use operit_edge_sdk::agent::*;
use std::{
    future::Future,
    pin::pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Wake, Waker},
};
struct ThreadWake(std::thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}
fn run<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWake(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}
struct Sensor(Arc<AtomicUsize>);
impl Tool for Sensor {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "sensor",
            description: "Read sensor",
            input_schema: "{}",
        }
    }
    fn call(&self, _: &str) -> Result<String, AgentError> {
        self.0.fetch_add(1, Ordering::Relaxed);
        Ok("23.5".into())
    }
}
struct Model {
    name: &'static str,
    repeat: bool,
}
impl Backend for Model {
    fn next<'a>(&'a self, request: Request<'a>) -> AgentFuture<'a> {
        Box::pin(async move {
            if !self.repeat && !request.observations.is_empty() {
                assert_eq!(request.observations[0].result, "23.5");
                return Ok(Step::Reply("Temperature is 23.5 C".into()));
            }
            Ok(Step::Call {
                tool: self.name.into(),
                arguments: "{}".into(),
            })
        })
    }
}
#[test]
fn local_tool_result_returns_to_the_model_before_the_reply() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut agent = Agent::new(
        Model {
            name: "sensor",
            repeat: false,
        },
        Limits::default(),
    )
    .unwrap();
    agent.register(Sensor(calls.clone())).unwrap();
    assert_eq!(
        run(agent.run("Answer from the sensor", "temperature")).unwrap(),
        "Temperature is 23.5 C"
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}
#[test]
fn unknown_tools_and_oversized_prompts_never_touch_hardware() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut agent = Agent::new(
        Model {
            name: "motor",
            repeat: false,
        },
        Limits::default(),
    )
    .unwrap();
    agent.register(Sensor(calls.clone())).unwrap();
    assert_eq!(
        run(agent.run("", "")),
        Err(AgentError::UnknownTool("motor".into()))
    );
    assert_eq!(
        run(agent.run("", &"x".repeat(2049))),
        Err(AgentError::InputTooLarge)
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}
#[test]
fn repeated_tool_calls_stop_at_the_step_budget() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut agent = Agent::new(
        Model {
            name: "sensor",
            repeat: true,
        },
        Limits {
            max_steps: 2,
            ..Limits::default()
        },
    )
    .unwrap();
    agent.register(Sensor(calls.clone())).unwrap();
    assert_eq!(run(agent.run("", "")), Err(AgentError::StepLimit));
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}
#[test]
fn context_exhaustion_is_checked_before_a_physical_action() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut agent = Agent::new(
        Model {
            name: "sensor",
            repeat: true,
        },
        Limits {
            max_context_bytes: 4,
            ..Limits::default()
        },
    )
    .unwrap();
    agent.register(Sensor(calls.clone())).unwrap();
    assert_eq!(run(agent.run("", "")), Err(AgentError::OutputTooLarge));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}
