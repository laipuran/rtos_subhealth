use ros_task_client::{ExecEndpointConfig, RosConnectionConfig, RosTaskError};
use std::time::Duration;

fn valid_config() -> RosConnectionConfig {
    RosConnectionConfig {
        node_name: "control_plane_server".into(),
        endpoints: vec![ExecEndpointConfig {
            device_id: "mock_exec".into(),
            action_name: "/mock_exec/execute_task".into(),
        }],
        feedback_buffer: 16,
        server_wait_timeout: Duration::from_secs(2),
    }
}

#[test]
fn valid_static_endpoint_configuration_is_accepted() {
    assert!(valid_config().validate().is_ok());
}

#[test]
fn duplicate_device_ids_and_action_names_are_rejected() {
    let mut ids = valid_config();
    ids.endpoints.push(ExecEndpointConfig {
        device_id: "mock_exec".into(),
        action_name: "/other/task".into(),
    });
    assert!(matches!(
        ids.validate(),
        Err(RosTaskError::InvalidConfig {
            field: "device_id",
            ..
        })
    ));

    let mut names = valid_config();
    names.endpoints.push(ExecEndpointConfig {
        device_id: "other".into(),
        action_name: "/mock_exec/execute_task".into(),
    });
    assert!(matches!(
        names.validate(),
        Err(RosTaskError::InvalidConfig {
            field: "action_name",
            ..
        })
    ));
}

#[test]
fn empty_names_zero_buffer_and_zero_timeout_are_rejected() {
    let mut node = valid_config();
    node.node_name.clear();
    assert_invalid_field(node.validate(), "node_name");
    let mut device = valid_config();
    device.endpoints[0].device_id.clear();
    assert_invalid_field(device.validate(), "device_id");
    let mut action = valid_config();
    action.endpoints[0].action_name.clear();
    assert_invalid_field(action.validate(), "action_name");
    let mut buffer = valid_config();
    buffer.feedback_buffer = 0;
    assert_invalid_field(buffer.validate(), "feedback_buffer");
    let mut timeout = valid_config();
    timeout.server_wait_timeout = Duration::ZERO;
    assert_invalid_field(timeout.validate(), "server_wait_timeout");
}

#[test]
fn leading_and_trailing_whitespace_in_names_is_rejected() {
    let mut node_leading = valid_config();
    node_leading.node_name.insert(0, ' ');
    assert_invalid_field(node_leading.validate(), "node_name");

    let mut node_trailing = valid_config();
    node_trailing.node_name.push(' ');
    assert_invalid_field(node_trailing.validate(), "node_name");

    let mut device_leading = valid_config();
    device_leading.endpoints[0].device_id.insert(0, '\t');
    assert_invalid_field(device_leading.validate(), "device_id");

    let mut device_trailing = valid_config();
    device_trailing.endpoints[0].device_id.push('\n');
    assert_invalid_field(device_trailing.validate(), "device_id");

    let mut action_leading = valid_config();
    action_leading.endpoints[0].action_name.insert(0, ' ');
    assert_invalid_field(action_leading.validate(), "action_name");

    let mut action_trailing = valid_config();
    action_trailing.endpoints[0].action_name.push('\t');
    assert_invalid_field(action_trailing.validate(), "action_name");
}

#[test]
fn action_names_must_be_canonical_absolute_names() {
    for invalid in [
        "/",
        "/bad-name",
        "/foo bar",
        "/1device/execute_task",
        "/device/execute.task",
        "/device/{execute_task}",
        "mock_exec/execute_task",
        "/mock_exec/execute_task/",
        "/mock_exec//execute_task",
    ] {
        let mut config = valid_config();
        config.endpoints[0].action_name = invalid.into();
        assert_invalid_field(config.validate(), "action_name");
    }
}

#[test]
fn valid_canonical_action_names_follow_ros_graph_name_grammar() {
    for valid in [
        "/mock_exec/execute_task",
        "/_mock_exec/execute_task_2",
        "/mock_exec_2/_execute_task",
    ] {
        let mut config = valid_config();
        config.endpoints[0].action_name = valid.into();
        assert!(config.validate().is_ok(), "{valid} should be valid");
    }
}

fn assert_invalid_field(result: Result<(), RosTaskError>, expected_field: &'static str) {
    assert!(matches!(
        result,
        Err(RosTaskError::InvalidConfig {
            field,
            message: _,
        }) if field == expected_field
    ));
}
