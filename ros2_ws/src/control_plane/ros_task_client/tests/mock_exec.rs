use std::{future::Future, time::Duration};

use ros_task_client::{
    ExecEndpointConfig, ExecuteCommand, PrimitiveCommand, RosConnectionConfig, RosTaskClient,
    RosTaskError, TaskSession,
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

async fn within<T>(future: impl Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(5), future)
        .await
        .expect("integration operation timed out")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires running mock_exec_layer"]
async fn go_to_tag_reports_overflow_when_feedback_is_not_drained() {
    let (client, runtime) = RosTaskClient::start(config("go_to_tag_overflow", 1)).unwrap();

    let session = within(client.execute(ExecuteCommand {
        task_id: "rust-go-to-tag".into(),
        device_id: "mock_exec".into(),
        primitive: PrimitiveCommand::GoToTag,
        target: vec![7],
        deadline_ms: None,
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
