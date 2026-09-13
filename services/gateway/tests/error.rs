use gateway::error::{ApiError, ErrorCode};

#[test]
fn error_codes_map_to_http_status() {
    assert_eq!(ApiError::new(ErrorCode::InvalidJson, "x").status(), 400);
    assert_eq!(ApiError::new(ErrorCode::InvalidGoal, "x").status(), 400);
    assert_eq!(ApiError::new(ErrorCode::InvalidParam, "x").status(), 400);
    assert_eq!(ApiError::new(ErrorCode::InvalidState, "x").status(), 400);
    assert_eq!(ApiError::new(ErrorCode::NotFound, "x").status(), 404);
    assert_eq!(ApiError::new(ErrorCode::Conflict, "x").status(), 409);
    assert_eq!(ApiError::new(ErrorCode::Unauthorized, "x").status(), 401);
    assert_eq!(ApiError::new(ErrorCode::Internal, "x").status(), 500);
}

#[test]
fn error_body_matches_rfc005_envelope() {
    let err = ApiError::new(ErrorCode::NotFound, "nope").with_details(serde_json::json!({"id": 1}));
    let body = err.to_body();
    assert_eq!(body["error"]["code"], "NOT_FOUND");
    assert_eq!(body["error"]["message"], "nope");
    assert_eq!(body["error"]["details"]["id"], 1);
}
