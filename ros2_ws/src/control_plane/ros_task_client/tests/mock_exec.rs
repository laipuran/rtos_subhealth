use std::{
    future::Future,
    process::{Child, Command, Stdio},
    time::Duration,
};

use ros_task_client::{
    ExecEndpointConfig, ExecuteCommand, FinalState, PrimitiveCommand, PrimitiveDetails,
    RosConnectionConfig, RosTaskClient, RosTaskError, TaskSession,
};

fn config(test_name: &str, feedback_buffer: usize) -> RosConnectionConfig {
    RosConnectionConfig {
        node_name: format!("ros_task_client_{test_name}_{}", std::process::id()),
        endpoints: vec![ExecEndpointConfig {
            device_id: "mock_exec".into(),
            action_name: "/mock_exec/execute_task".into(),
        }],
        feedback_buffer,
        server_wait_timeout: Duration::from_secs(2),
    }
}

struct MockProcess(Child);

async fn within<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("integration operation timed out")
}

async fn within_long<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(40), future)
        .await
        .expect("long integration operation timed out")
}

impl Drop for MockProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn hold_relays_feedback_and_successful_result() {
    let (client, runtime) = RosTaskClient::start(config("hold_success", 8)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-hold".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::Hold,
        deadline_unix_ms: None,
    }))
    .await
    .unwrap();
    let TaskSession {
        mut feedback,
        result,
        ..
    } = session;

    assert_eq!(
        within(feedback.recv()).await.unwrap().unwrap().details,
        PrimitiveDetails::Hold
    );
    assert_eq!(
        within(result).await.unwrap().unwrap().final_state,
        FinalState::Succeeded
    );
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn cloned_clients_submit_independently() {
    let (client, runtime) = RosTaskClient::start(config("clone_submit", 8)).unwrap();
    let cloned = client.clone();

    for (client, task_id) in [(cloned, "rust-clone"), (client, "rust-original")] {
        let session = within(client.execute(ExecuteCommand {
            task_id: task_id.into(),
            device_id: "mock_exec".into(),
            primitive: PrimitiveCommand::Hold,
            deadline_unix_ms: None,
        }))
        .await
        .unwrap();
        assert_eq!(
            within(session.result).await.unwrap().unwrap().task_id,
            task_id
        );
    }

    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn go_to_tag_failure_preserves_sdk_error_result() {
    let (client, runtime) = RosTaskClient::start(config("go_to_tag_failure", 8)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-failure".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag { target_tag: 42 },
        deadline_unix_ms: None,
    }))
    .await
    .unwrap();

    let result = within(session.result).await.unwrap().unwrap();
    assert_eq!(result.final_state, FinalState::Failed);
    assert_eq!(result.error_code.as_deref(), Some("SDK_ERROR"));
    assert_eq!(result.task_id, "rust-failure");
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn cancellation_waits_for_acceptance_and_terminal_canceled_result() {
    let (client, runtime) = RosTaskClient::start(config("go_to_tag_cancel", 8)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-canceled".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag { target_tag: 7 },
        deadline_unix_ms: None,
    }))
    .await
    .unwrap();
    let TaskSession {
        mut feedback,
        result,
        cancellation,
        ..
    } = session;

    let first = within(feedback.recv()).await.unwrap().unwrap();
    assert_eq!(first.task_id, "rust-canceled");
    assert_eq!(within(cancellation.cancel()).await, Ok(()));
    assert_eq!(
        within(result).await.unwrap().unwrap().final_state,
        FinalState::Canceled
    );

    within(async {
        while let Some(update) = feedback.recv().await {
            let update = update.unwrap();
            if update.phase == "canceled" {
                assert_eq!(update.state, ros_task_client::FeedbackState::Canceled);
            }
        }
    })
    .await;
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn go_to_tag_reports_overflow_when_feedback_is_not_drained() {
    let (client, runtime) = RosTaskClient::start(config("go_to_tag_overflow", 1)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-go-to-tag".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag { target_tag: 7 },
        deadline_unix_ms: None,
    }))
    .await
    .unwrap();
    let TaskSession {
        feedback, result, ..
    } = session;

    let outcome = tokio::time::timeout(Duration::from_secs(2), result)
        .await
        .expect("overflow result timed out")
        .unwrap();
    assert!(matches!(
        outcome,
        Err(RosTaskError::FeedbackOverflow {
            task_id,
            capacity: 1,
        }) if task_id == "rust-go-to-tag"
    ));
    drop(feedback);
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn go_to_tag_draining_feedback_delivers_final_update_and_success() {
    let (client, runtime) = RosTaskClient::start(config("go_to_tag_success", 8)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-go-to-tag-drained".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag { target_tag: 7 },
        deadline_unix_ms: None,
    }))
    .await
    .unwrap();
    let TaskSession {
        mut feedback,
        result,
        ..
    } = session;

    let mut final_details = None;
    within(async {
        while let Some(update) = feedback.recv().await {
            final_details = Some(update.unwrap().details);
        }
    })
    .await;
    assert_eq!(
        final_details,
        Some(PrimitiveDetails::GoToTag {
            current_tag: Some(7),
            next_tag: None,
        })
    );
    assert_eq!(
        within(result).await.unwrap().unwrap().final_state,
        FinalState::Succeeded
    );
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn unavailable_server_times_out_without_becoming_goal_rejection() {
    let mut unavailable = config("unavailable", 1);
    unavailable.endpoints[0].action_name = "/missing/execute_task".into();
    unavailable.server_wait_timeout = Duration::from_millis(100);
    let (client, runtime) = RosTaskClient::start(unavailable).unwrap();

    let outcome = within(client.execute(ExecuteCommand {
        task_id: "rust-unavailable".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::Hold,
        deadline_unix_ms: None,
    }))
    .await;

    assert!(matches!(
        outcome,
        Err(RosTaskError::ActionServerUnavailable { .. })
    ));
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires mock_exec_layer executable"]
async fn waits_for_server_that_appears_after_execution_starts() {
    let mut late = config("late_server", 8);
    late.endpoints[0].device_id = "late_mock".into();
    late.endpoints[0].action_name = "/late_mock/execute_task".into();
    late.server_wait_timeout = Duration::from_secs(30);
    let (client, runtime) = RosTaskClient::start(late).unwrap();

    let mock_executable = std::env::var("MOCK_EXECUTABLE")
        .expect("MOCK_EXECUTABLE must name the mock action server executable");
    let delayed_server = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        MockProcess(
            Command::new(mock_executable)
                .args([
                    "--ros-args",
                    "-r",
                    "__node:=late_mock_layer",
                    "-p",
                    "action_name:=/late_mock/execute_task",
                    "-p",
                    "device_id:=late_mock",
                    "-p",
                    "step_delay_s:=0.01",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("failed to launch delayed mock action server"),
        )
    });

    let session = within_long(client.execute(ExecuteCommand {
        task_id: "rust-late-server".into(),
        device_id: "late_mock".into(),
        primitive: PrimitiveCommand::Hold,
        deadline_unix_ms: None,
    }))
    .await
    .expect("client did not wait for delayed action server");
    let mut feedback = session.feedback;
    assert_eq!(
        within(feedback.recv()).await.unwrap().unwrap().details,
        PrimitiveDetails::Hold
    );
    assert_eq!(
        within(session.result).await.unwrap().unwrap().final_state,
        FinalState::Succeeded
    );

    let delayed_server = within(delayed_server).await.unwrap();
    drop(delayed_server);
    runtime.shutdown().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn available_server_rejection_is_not_reported_as_unavailable() {
    let mut rejected = config("rejected", 1);
    rejected.endpoints[0].device_id = "wrong_device".into();
    let (client, runtime) = RosTaskClient::start(rejected).unwrap();

    let outcome = within(client.execute(ExecuteCommand {
        task_id: "rust-rejected".into(),
        device_id: "wrong_device".into(),
        primitive: PrimitiveCommand::Hold,
        deadline_unix_ms: None,
    }))
    .await;

    assert!(matches!(outcome, Err(RosTaskError::GoalRejected { .. })));
    runtime.shutdown().unwrap();
}
