use std::{
    collections::HashMap,
    fmt,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use rclrs::{
    ActionClient, CancelResponseClient, CancelResponseCode, CreateBasicExecutor, GoalClient, Node,
    TopicNamesAndTypes,
};
use ros_env::task_interfaces::action::{ExecuteTask, ExecuteTask_Feedback};
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::Instant,
};

use crate::{
    mapper::{from_ros_feedback, from_ros_result, to_ros_goal},
    types::CancelRequest,
    CancellationHandle, ExecuteCommand, RosConnectionConfig, RosTaskError, RosTaskRuntime,
    TaskSession,
};

type PendingCancel = (
    CancelResponseClient<ExecuteTask>,
    oneshot::Sender<Result<(), RosTaskError>>,
);

enum RelayState {
    Receiving,
    AwaitingCancel(PendingCancel),
}

enum RelayEvent<Feedback, Result> {
    Feedback(Feedback),
    Result(Result),
}

enum RelayInput<Request, Event> {
    Request(Request),
    Event(Event),
}

struct Endpoint {
    action_name: String,
    action_client: ActionClient<ExecuteTask>,
}

const ACTION_POLL_INTERVAL: Duration = Duration::from_millis(25);
const SEND_GOAL_TYPE: &str = "task_interfaces/action/ExecuteTask_SendGoal";
const GET_RESULT_TYPE: &str = "task_interfaces/action/ExecuteTask_GetResult";
const CANCEL_GOAL_TYPE: &str = "action_msgs/srv/CancelGoal";
const FEEDBACK_TYPE: &str = "task_interfaces/action/ExecuteTask_FeedbackMessage";
const STATUS_TYPE: &str = "action_msgs/msg/GoalStatusArray";
const READY_FEEDBACK_DRAIN_LIMIT: usize = 16;

struct ClientState {
    node: Node,
    endpoints: HashMap<String, Endpoint>,
    stopping: Arc<AtomicBool>,
    shutdown_rx: watch::Receiver<bool>,
    feedback_buffer: usize,
    server_wait_timeout: std::time::Duration,
}

#[derive(Clone)]
pub struct RosTaskClient {
    state: Arc<ClientState>,
}

impl RosTaskClient {
    pub fn start(
        config: RosConnectionConfig,
    ) -> Result<(RosTaskClient, RosTaskRuntime), RosTaskError> {
        config.validate()?;

        let context = rclrs::Context::default_from_env().map_err(ros_error)?;
        let executor = context.create_basic_executor();
        let node = executor
            .create_node(config.node_name.as_str())
            .map_err(ros_error)?;

        let mut endpoints = HashMap::with_capacity(config.endpoints.len());
        for endpoint in &config.endpoints {
            let action_client = node
                .create_action_client::<ExecuteTask>(&endpoint.action_name)
                .map_err(ros_error)?;
            endpoints.insert(
                endpoint.device_id.clone(),
                Endpoint {
                    action_name: endpoint.action_name.clone(),
                    action_client,
                },
            );
        }

        let stopping = Arc::new(AtomicBool::new(false));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let runtime = RosTaskRuntime::start(executor, Arc::clone(&stopping), shutdown_tx)?;
        let client = RosTaskClient {
            state: Arc::new(ClientState {
                node,
                endpoints,
                stopping,
                shutdown_rx,
                feedback_buffer: config.feedback_buffer,
                server_wait_timeout: config.server_wait_timeout,
            }),
        };

        Ok((client, runtime))
    }

    pub async fn execute(&self, command: ExecuteCommand) -> Result<TaskSession, RosTaskError> {
        let state = &self.state;
        if state.stopping.load(Ordering::Acquire) || *state.shutdown_rx.borrow() {
            return Err(RosTaskError::Shutdown);
        }

        let endpoint =
            state
                .endpoints
                .get(&command.device_id)
                .ok_or_else(|| RosTaskError::UnknownDevice {
                    device_id: command.device_id.clone(),
                })?;
        let raw_goal = to_ros_goal(&command)?;
        let action_client = Arc::clone(&endpoint.action_client);
        let action_name = endpoint.action_name.clone();
        let device_id = command.device_id.clone();
        let task_id = command.task_id.clone();
        let primitive = command.primitive.clone();
        let mut shutdown_rx = state.shutdown_rx.clone();
        let request_deadline = Instant::now() + state.server_wait_timeout;

        loop {
            if state.stopping.load(Ordering::Acquire) || *shutdown_rx.borrow() {
                return Err(RosTaskError::Shutdown);
            }
            if action_server_is_ready(&state.node, &action_name).map_err(ros_error)? {
                break;
            }

            let now = Instant::now();
            if now >= request_deadline {
                return Err(RosTaskError::ActionServerUnavailable {
                    device_id,
                    action_name,
                });
            }
            let delay = ACTION_POLL_INTERVAL.min(request_deadline - now);
            tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return Err(RosTaskError::Shutdown);
                    }
                }
                () = tokio::time::sleep(delay) => {}
            }
        }
        if Instant::now() >= request_deadline {
            return Err(RosTaskError::ActionServerUnavailable {
                device_id,
                action_name,
            });
        }

        let requested_goal = action_client
            .try_request_goal(raw_goal)
            .map_err(ros_error)?;
        tokio::pin!(requested_goal);
        let accepted_goal = loop {
            tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        return Err(RosTaskError::Shutdown);
                    }
                }
                accepted = tokio::time::timeout_at(request_deadline, &mut requested_goal) => {
                    break accepted.map_err(|_| RosTaskError::ActionServerUnavailable {
                        device_id: device_id.clone(),
                        action_name: action_name.clone(),
                    })?;
                }
            }
        };
        let goal = accepted_goal.ok_or_else(|| RosTaskError::GoalRejected {
            task_id: task_id.clone(),
        })?;

        let (feedback_tx, feedback_rx) = mpsc::channel(state.feedback_buffer);
        let (result_tx, result_rx) = oneshot::channel();
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<CancelRequest>(1);
        let cancellation_handle = CancellationHandle::new(cancel_tx, task_id.clone());
        let mut relay_shutdown_rx = state.shutdown_rx.clone();
        let relay_task_id = task_id.clone();

        tokio::spawn(async move {
            let GoalClient {
                mut feedback,
                result,
                cancellation,
                ..
            } = goal;
            tokio::pin!(result);
            let mut transport_feedback_open = true;
            let mut cancel_open = true;
            let mut relay_state = RelayState::Receiving;

            if *relay_shutdown_rx.borrow() {
                let _ = result_tx.send(Err(RosTaskError::Shutdown));
                return;
            }

            let terminal_result = loop {
                match std::mem::replace(&mut relay_state, RelayState::Receiving) {
                    RelayState::AwaitingCancel(mut pending_cancel) => {
                        tokio::select! {
                            biased;
                            changed = relay_shutdown_rx.changed() => {
                                let _ = pending_cancel.1.send(cancel_unavailable(
                                    &relay_task_id,
                                    "relay shutdown made the cancellation response unavailable",
                                ));
                                let _ = changed;
                                break Err(RosTaskError::Shutdown);
                            }
                            cancel_response = &mut pending_cancel.0 => {
                                let response = map_cancel_response(&relay_task_id, cancel_response.code);
                                let _ = pending_cancel.1.send(response);
                                relay_state = RelayState::Receiving;
                            }
                            terminal = result.as_mut() => {
                                let _ = pending_cancel.1.send(cancel_unavailable(
                                    &relay_task_id,
                                    "the terminal result became ready before the cancellation response",
                                ));
                                if let Err(error) = drain_ready_feedback(
                                    &mut feedback,
                                    &feedback_tx,
                                    feedback_tx.max_capacity(),
                                    &relay_task_id,
                                    &primitive,
                                ) {
                                    break Err(error);
                                }
                                break from_ros_result(&relay_task_id, terminal.0, terminal.1);
                            }
                        }
                    }
                    RelayState::Receiving => {
                        tokio::select! {
                            biased;
                            changed = relay_shutdown_rx.changed() => {
                                let _ = changed;
                                break Err(RosTaskError::Shutdown);
                            }
                            input = select_cancel_before_event(
                                async {
                                    if cancel_open {
                                        cancel_rx.recv().await
                                    } else {
                                        std::future::pending().await
                                    }
                                },
                                select_result_before_feedback(
                                    async {
                                        if transport_feedback_open {
                                            feedback.recv().await
                                        } else {
                                            std::future::pending().await
                                        }
                                    },
                                    result.as_mut(),
                                ),
                            ) => {
                                match input {
                                    RelayInput::Request(request) => {
                                        match request {
                                            Some(CancelRequest { response_tx }) => {
                                                match cancellation.try_cancel() {
                                                    Ok(response) => {
                                                        relay_state = RelayState::AwaitingCancel((
                                                            response,
                                                            response_tx,
                                                        ));
                                                    }
                                                    Err(error) => {
                                                        let _ = response_tx.send(Err(ros_error(error)));
                                                    }
                                                }
                                            }
                                            None => cancel_open = false,
                                        }
                                    }
                                    RelayInput::Event(event) => {
                                        match event {
                                            RelayEvent::Feedback(Some(raw_feedback)) => {
                                                if let Err(error) = map_and_try_send_feedback(
                                                    &feedback_tx,
                                                    feedback_tx.max_capacity(),
                                                    &relay_task_id,
                                                    &primitive,
                                                    raw_feedback,
                                                ) {
                                                    break Err(error);
                                                }
                                            }
                                            RelayEvent::Feedback(None) => {
                                                transport_feedback_open = false;
                                            }
                                            RelayEvent::Result((status, raw_result)) => {
                                                if let Err(error) = drain_ready_feedback(
                                                    &mut feedback,
                                                    &feedback_tx,
                                                    feedback_tx.max_capacity(),
                                                    &relay_task_id,
                                                    &primitive,
                                                ) {
                                                    break Err(error);
                                                }
                                                break from_ros_result(
                                                    &relay_task_id,
                                                    status,
                                                    raw_result,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            };

            while let Ok(CancelRequest { response_tx }) = cancel_rx.try_recv() {
                let _ = response_tx.send(cancel_unavailable(
                    &relay_task_id,
                    "the relay completed before the cancellation request was accepted",
                ));
            }

            let _ = result_tx.send(terminal_result);
        });

        Ok(TaskSession {
            task_id,
            feedback: feedback_rx,
            result: result_rx,
            cancellation: cancellation_handle,
        })
    }
}

async fn select_result_before_feedback<FeedbackFuture, ResultFuture>(
    feedback: FeedbackFuture,
    result: Pin<&mut ResultFuture>,
) -> RelayEvent<FeedbackFuture::Output, ResultFuture::Output>
where
    FeedbackFuture: Future,
    ResultFuture: Future + ?Sized,
{
    tokio::select! {
        biased;
        result = result => RelayEvent::Result(result),
        feedback = feedback => RelayEvent::Feedback(feedback),
    }
}

async fn select_cancel_before_event<CancelFuture, EventFuture>(
    cancel: CancelFuture,
    event: EventFuture,
) -> RelayInput<CancelFuture::Output, EventFuture::Output>
where
    CancelFuture: Future,
    EventFuture: Future,
{
    tokio::select! {
        biased;
        request = cancel => RelayInput::Request(request),
        event = event => RelayInput::Event(event),
    }
}

fn drain_ready_feedback(
    feedback: &mut tokio::sync::mpsc::UnboundedReceiver<ExecuteTask_Feedback>,
    feedback_tx: &mpsc::Sender<Result<crate::TaskFeedback, RosTaskError>>,
    capacity: usize,
    task_id: &str,
    primitive: &crate::PrimitiveCommand,
) -> Result<(), RosTaskError> {
    for _ in 0..READY_FEEDBACK_DRAIN_LIMIT {
        let raw_feedback = match feedback.try_recv() {
            Ok(raw_feedback) => raw_feedback,
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
            | Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => break,
        };
        map_and_try_send_feedback(feedback_tx, capacity, task_id, primitive, raw_feedback)?;
    }
    Ok(())
}

fn map_and_try_send_feedback(
    feedback_tx: &mpsc::Sender<Result<crate::TaskFeedback, RosTaskError>>,
    capacity: usize,
    task_id: &str,
    primitive: &crate::PrimitiveCommand,
    raw: ExecuteTask_Feedback,
) -> Result<(), RosTaskError> {
    let mapped = from_ros_feedback(task_id, primitive, raw)?;
    match feedback_tx.try_send(Ok(mapped)) {
        Ok(()) => Ok(()),
        Err(mpsc::error::TrySendError::Full(_)) => Err(RosTaskError::FeedbackOverflow {
            task_id: task_id.into(),
            capacity,
        }),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(RosTaskError::ChannelClosed {
            channel: "feedback",
        }),
    }
}

fn map_cancel_response(task_id: &str, code: CancelResponseCode) -> Result<(), RosTaskError> {
    match code {
        CancelResponseCode::Accept => Ok(()),
        code => Err(RosTaskError::CancelRejected {
            task_id: task_id.into(),
            reason: format!("{code:?}"),
        }),
    }
}

fn cancel_unavailable(task_id: &str, reason: &str) -> Result<(), RosTaskError> {
    Err(RosTaskError::CancelRejected {
        task_id: task_id.into(),
        reason: reason.into(),
    })
}

fn action_server_is_ready(node: &Node, action_name: &str) -> Result<bool, rclrs::RclrsError> {
    let mut services = TopicNamesAndTypes::new();
    let mut publishers = TopicNamesAndTypes::new();

    for server_node in node.get_node_names()? {
        if let Ok(node_services) =
            node.get_service_names_and_types_by_node(&server_node.name, &server_node.namespace)
        {
            merge_names_and_types(&mut services, node_services);
        }
        if let Ok(node_publishers) =
            node.get_publisher_names_and_types_by_node(&server_node.name, &server_node.namespace)
        {
            merge_names_and_types(&mut publishers, node_publishers);
        }
    }

    Ok(action_entities_are_ready(
        action_name,
        &services,
        &publishers,
    ))
}

fn merge_names_and_types(target: &mut TopicNamesAndTypes, source: TopicNamesAndTypes) {
    for (name, types) in source {
        target.entry(name).or_default().extend(types);
    }
}

fn action_entities_are_ready(
    action_name: &str,
    services: &TopicNamesAndTypes,
    publishers: &TopicNamesAndTypes,
) -> bool {
    has_type(
        services,
        &format!("{action_name}/_action/send_goal"),
        SEND_GOAL_TYPE,
    ) && has_type(
        services,
        &format!("{action_name}/_action/get_result"),
        GET_RESULT_TYPE,
    ) && has_type(
        services,
        &format!("{action_name}/_action/cancel_goal"),
        CANCEL_GOAL_TYPE,
    ) && has_type(
        publishers,
        &format!("{action_name}/_action/feedback"),
        FEEDBACK_TYPE,
    ) && has_type(
        publishers,
        &format!("{action_name}/_action/status"),
        STATUS_TYPE,
    )
}

fn has_type(names_and_types: &TopicNamesAndTypes, name: &str, expected: &str) -> bool {
    names_and_types
        .get(name)
        .is_some_and(|types| types.iter().any(|actual| actual == expected))
}

impl fmt::Debug for RosTaskClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RosTaskClient")
            .field("endpoint_count", &self.state.endpoints.len())
            .field(
                "stopping",
                &self
                    .state
                    .stopping
                    .load(std::sync::atomic::Ordering::Acquire),
            )
            .field("shutdown_requested", &*self.state.shutdown_rx.borrow())
            .field("feedback_buffer", &self.state.feedback_buffer)
            .field("server_wait_timeout", &self.state.server_wait_timeout)
            .finish_non_exhaustive()
    }
}

fn ros_error(error: rclrs::RclrsError) -> RosTaskError {
    RosTaskError::Ros(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rclrs::CancelResponseCode;
    use ros_env::{builtin_interfaces::msg::Time, task_interfaces::action::ExecuteTask_Feedback};
    use tokio::{
        sync::mpsc,
        time::{sleep, timeout},
    };

    use crate::{PrimitiveCommand, RosTaskError};

    use super::{
        action_entities_are_ready, drain_ready_feedback, map_and_try_send_feedback,
        map_cancel_response, select_cancel_before_event, select_result_before_feedback, RelayEvent,
        RelayInput, RosTaskClient,
    };

    #[test]
    fn client_is_cloneable_for_independent_submissions() {
        fn assert_clone<T: Clone>() {}

        assert_clone::<RosTaskClient>();
    }

    #[test]
    fn cancel_response_mapping_distinguishes_acceptance_from_rejection() {
        assert_eq!(
            map_cancel_response("task-1", CancelResponseCode::Accept),
            Ok(())
        );

        for code in [
            CancelResponseCode::Reject,
            CancelResponseCode::UnknownGoal,
            CancelResponseCode::GoalTerminated,
        ] {
            assert!(matches!(
                map_cancel_response("task-1", code),
                Err(RosTaskError::CancelRejected { task_id, reason })
                    if task_id == "task-1" && reason == format!("{code:?}")
            ));
        }
    }

    #[test]
    fn feedback_mapping_error_wins_over_full_channel() {
        let primitive = PrimitiveCommand::GoToTag { target_tag: 7 };
        let (feedback_tx, _feedback_rx) = mpsc::channel(1);
        let valid = ExecuteTask_Feedback {
            task_id: "task-1".into(),
            state: "running".into(),
            progress: 0.5,
            phase: "moving".into(),
            details_json: r#"{"current_tag":-1,"next_tag":7}"#.into(),
            timestamp: Time::default(),
        };
        map_and_try_send_feedback(&feedback_tx, 1, "task-1", &primitive, valid).unwrap();

        let malformed = ExecuteTask_Feedback {
            task_id: "task-1".into(),
            state: "running".into(),
            progress: 1.0,
            phase: "moving".into(),
            details_json: r#"{"current_tag":7}"#.into(),
            timestamp: Time::default(),
        };
        let outcome = map_and_try_send_feedback(&feedback_tx, 1, "task-1", &primitive, malformed);

        assert!(matches!(
            outcome,
            Err(RosTaskError::Mapping {
                field: "details_json",
                ..
            })
        ));
    }

    #[test]
    fn full_feedback_channel_returns_typed_overflow() {
        let primitive = PrimitiveCommand::GoToTag { target_tag: 7 };
        let (feedback_tx, _feedback_rx) = mpsc::channel(1);
        let feedback = || ExecuteTask_Feedback {
            task_id: "task-1".into(),
            state: "running".into(),
            progress: 0.5,
            phase: "moving".into(),
            details_json: r#"{"current_tag":-1,"next_tag":7}"#.into(),
            timestamp: Time::default(),
        };
        map_and_try_send_feedback(&feedback_tx, 1, "task-1", &primitive, feedback()).unwrap();

        assert!(matches!(
            map_and_try_send_feedback(&feedback_tx, 1, "task-1", &primitive, feedback()),
            Err(RosTaskError::FeedbackOverflow {
                task_id,
                capacity: 1,
            }) if task_id == "task-1"
        ));
    }

    #[test]
    fn closed_feedback_channel_returns_channel_closed() {
        let primitive = PrimitiveCommand::GoToTag { target_tag: 7 };
        let (feedback_tx, feedback_rx) = mpsc::channel(1);
        drop(feedback_rx);
        let feedback = ExecuteTask_Feedback {
            task_id: "task-1".into(),
            state: "running".into(),
            progress: 0.5,
            phase: "moving".into(),
            details_json: r#"{"current_tag":-1,"next_tag":7}"#.into(),
            timestamp: Time::default(),
        };

        assert!(matches!(
            map_and_try_send_feedback(&feedback_tx, 1, "task-1", &primitive, feedback),
            Err(RosTaskError::ChannelClosed {
                channel: "feedback"
            })
        ));
    }

    #[tokio::test]
    async fn continuously_ready_feedback_cannot_starve_result() {
        let (public_feedback_tx, mut public_feedback_rx) = mpsc::channel(1);
        let drain = tokio::spawn(async move { while public_feedback_rx.recv().await.is_some() {} });
        let mut result = Box::pin(async {
            sleep(std::time::Duration::from_millis(10)).await;
            7
        });

        let delivered_result = timeout(std::time::Duration::from_secs(1), async {
            loop {
                match select_result_before_feedback(std::future::ready(Some(())), result.as_mut())
                    .await
                {
                    RelayEvent::Feedback(Some(feedback)) => {
                        public_feedback_tx.send(feedback).await.unwrap();
                    }
                    RelayEvent::Feedback(None) => panic!("feedback source ended unexpectedly"),
                    RelayEvent::Result(result) => break result,
                }
            }
        })
        .await
        .expect("result was starved by continuously-ready feedback");

        assert_eq!(delivered_result, 7);
        drop(public_feedback_tx);
        drain.await.unwrap();
    }

    #[tokio::test]
    async fn result_drains_simultaneously_ready_terminal_feedback() {
        let (raw_feedback_tx, mut raw_feedback_rx) = mpsc::unbounded_channel();
        raw_feedback_tx
            .send(ExecuteTask_Feedback {
                task_id: "task-1".into(),
                state: "running".into(),
                progress: 1.0,
                phase: "complete".into(),
                details_json: r#"{"current_tag":7,"next_tag":-1}"#.into(),
                timestamp: Time::default(),
            })
            .unwrap();

        let (public_feedback_tx, mut public_feedback_rx) = mpsc::channel(1);
        let mut result = Box::pin(std::future::ready(7));
        let selected = select_result_before_feedback(raw_feedback_rx.recv(), result.as_mut()).await;
        assert!(matches!(selected, RelayEvent::Result(7)));

        drain_ready_feedback(
            &mut raw_feedback_rx,
            &public_feedback_tx,
            1,
            "task-1",
            &PrimitiveCommand::GoToTag { target_tag: 7 },
        )
        .unwrap();

        let feedback = public_feedback_rx.recv().await.unwrap().unwrap();
        assert_eq!(
            feedback.details,
            crate::PrimitiveDetails::GoToTag {
                current_tag: Some(7),
                next_tag: None,
            }
        );
    }

    #[tokio::test]
    async fn continuously_ready_feedback_cannot_starve_cancellation_request() {
        for _ in 0..100 {
            let selected = select_cancel_before_event(
                std::future::ready(Some("cancel")),
                std::future::ready("feedback"),
            )
            .await;
            assert!(matches!(selected, RelayInput::Request(Some("cancel"))));
        }
    }

    #[test]
    fn action_readiness_requires_all_entities_with_exact_types() {
        let action_name = "/mock_exec/execute_task";
        let mut services = HashMap::from([
            (
                format!("{action_name}/_action/send_goal"),
                vec!["task_interfaces/action/ExecuteTask_SendGoal".into()],
            ),
            (
                format!("{action_name}/_action/get_result"),
                vec!["task_interfaces/action/ExecuteTask_GetResult".into()],
            ),
            (
                format!("{action_name}/_action/cancel_goal"),
                vec!["action_msgs/srv/CancelGoal".into()],
            ),
        ]);
        let mut publishers = HashMap::from([
            (
                format!("{action_name}/_action/feedback"),
                vec!["task_interfaces/action/ExecuteTask_FeedbackMessage".into()],
            ),
            (
                format!("{action_name}/_action/status"),
                vec!["action_msgs/msg/GoalStatusArray".into()],
            ),
        ]);

        assert!(action_entities_are_ready(
            action_name,
            &services,
            &publishers
        ));
        publishers.remove(&format!("{action_name}/_action/status"));
        assert!(!action_entities_are_ready(
            action_name,
            &services,
            &publishers
        ));
        publishers.insert(
            format!("{action_name}/_action/status"),
            vec!["action_msgs/msg/GoalStatusArray".into()],
        );
        services.insert(
            format!("{action_name}/_action/send_goal"),
            vec!["wrong/type".into()],
        );
        assert!(!action_entities_are_ready(
            action_name,
            &services,
            &publishers
        ));
    }
}
