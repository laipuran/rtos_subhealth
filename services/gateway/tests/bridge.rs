use gateway::bridge::mock::MockBridge;
use gateway::bridge::{BridgeCommand, RosBridge};
use gateway::model::task::Goal;

#[test]
fn mock_bridge_records_commands() {
    let bridge = MockBridge::new();
    bridge.send_goal("T-1", Goal::new("go_to_tag").with_tags(vec![42]));
    bridge.cancel("T-1");
    bridge.trigger_diagnosis("manual:abc");

    let cmds = bridge.commands();
    assert_eq!(cmds.len(), 3);
    match &cmds[0] {
        BridgeCommand::SendGoal { goal_id, goal } => {
            assert_eq!(goal_id, "T-1");
            assert_eq!(goal.target_tags, vec![42]);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(
        cmds[1],
        BridgeCommand::Cancel {
            goal_id: "T-1".into()
        }
    );
    assert_eq!(
        cmds[2],
        BridgeCommand::TriggerDiagnosis {
            diagnosis_id: "manual:abc".into()
        }
    );
    assert!(bridge.take().len() == 3);
    assert!(bridge.commands().is_empty());
}
