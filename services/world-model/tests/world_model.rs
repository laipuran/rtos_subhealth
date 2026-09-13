use world_model::{PlanError, Planner, WorldModel};

const MAP: &str = r#"{
  "tags": {
    "1": {"name": "a", "x": 0.0, "y": 0.0},
    "2": {"name": "b", "x": 1.0, "y": 0.0},
    "3": {"name": "c", "x": 2.0, "y": 0.0},
    "4": {"name": "d", "x": 0.0, "y": 1.0}
  },
  "edges": [
    {"from": 1, "to": 2, "cost": 1.0},
    {"from": 2, "to": 3, "cost": 1.0},
    {"from": 1, "to": 3, "cost": 5.0},
    {"from": 1, "to": 4, "cost": 1.0}
  ],
  "routes": {"r1": [1, 3], "r2": [1, 3, 4]}
}"#;

#[test]
fn parses_map() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    assert_eq!(model.nodes.len(), 4);
    assert_eq!(model.edges.len(), 4);
    assert_eq!(model.routes.get("r1").unwrap(), &vec![1, 3]);
    assert_eq!(model.nodes.get(&2).unwrap().name, "b");
}

#[test]
fn picks_cheapest_path() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    let segments = planner.shortest_path(1, 3, &[]).unwrap();
    let hops: Vec<(i32, i32)> = segments.iter().map(|s| (s.from, s.to)).collect();
    assert_eq!(hops, vec![(1, 2), (2, 3)]);
    let total: f64 = segments.iter().map(|s| s.cost).sum();
    assert!((total - 2.0).abs() < 1e-9);
}

#[test]
fn avoid_forces_detour() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    let segments = planner.shortest_path(1, 3, &[2]).unwrap();
    let hops: Vec<(i32, i32)> = segments.iter().map(|s| (s.from, s.to)).collect();
    assert_eq!(hops, vec![(1, 3)]);
}

#[test]
fn same_node_is_empty_path() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    assert!(planner.shortest_path(1, 1, &[]).unwrap().is_empty());
}

#[test]
fn unknown_node_is_error() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    assert_eq!(
        planner.shortest_path(9, 3, &[]),
        Err(PlanError::UnknownNode(9))
    );
    assert_eq!(
        planner.shortest_path(1, 9, &[]),
        Err(PlanError::UnknownNode(9))
    );
}

#[test]
fn no_route_is_error() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    assert_eq!(
        planner.shortest_path(3, 1, &[]),
        Err(PlanError::NoRoute { from: 3, to: 1 })
    );
}

#[test]
fn route_expands_to_segments() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    let segments = planner.route("r1", &[]).unwrap();
    let hops: Vec<(i32, i32)> = segments.iter().map(|s| (s.from, s.to)).collect();
    assert_eq!(hops, vec![(1, 2), (2, 3)]);
}

#[test]
fn route_with_unreachable_hop_is_error() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    // r2 = [1, 3, 4] and there is no 3 -> 4 edge.
    assert_eq!(
        planner.route("r2", &[]),
        Err(PlanError::NoRoute { from: 3, to: 4 })
    );
}

#[test]
fn unknown_route_is_error() {
    let model = WorldModel::from_json_str(MAP).unwrap();
    let planner = Planner::new(&model);
    assert_eq!(
        planner.route("nope", &[]),
        Err(PlanError::UnknownRoute("nope".into()))
    );
}
