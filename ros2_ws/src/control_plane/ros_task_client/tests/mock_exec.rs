use std::{future::Future, time::Duration};

use platform::{DeviceId, ExecutionError, ExecutionSession, Primitive, Task, TaskId};
use ros_task_client::{ExecEndpointConfig, RosConnectionConfig, RosTaskClient};

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

    let session = within(client.execute(Task {
        id: TaskId("rust-go-to-tag".into()),
        device_id: DeviceId("mock_exec".into()),
        primitive: Primitive::GoToTag,
        target: vec![7],
        deadline_ms: None,
    }))
    .await
    .unwrap();
    let ExecutionSession {
        feedback, result, ..
    } = session;

    let outcome = tokio::time::timeout(Duration::from_secs(2), result)
        .await
        .expect("overflow result timed out");
    assert!(matches!(
        outcome,
        Err(ExecutionError::Failed(message))
            if message == "feedback overflow for task rust-go-to-tag: channel capacity is 1"
    ));
    drop(feedback);
    runtime.shutdown().unwrap();
}
