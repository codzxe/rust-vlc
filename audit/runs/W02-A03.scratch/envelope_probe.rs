// W02-A03 probe: envelope wire forms — does the serde output match the
// documented §5.1 shapes (camelCase, "gen" rename, bare outcome / error envelope)?
use commands::CommandOutcome;
use commands::CommandResponse;
use commands::CommandResult;
use commands::Generation;
use commands::RequestId;
use commands::SessionId;
use commands::WireRequest;
use commands::catalog::types::OpenOptions;

fn main() {
    // 1. Accepted response.
    let ok: CommandResponse<CommandResult> = CommandResponse {
        request_id: RequestId(42),
        session: Some(SessionId(7)),
        generation: Generation(3),
        result: Ok(CommandOutcome::Accepted {
            session: Some(SessionId(7)),
            generation: Generation(4),
        }),
    };
    println!("accepted: {}", serde_json::to_string(&ok).unwrap());
    // 2. session: None must be skipped.
    let none_sess: CommandResponse<CommandResult> = CommandResponse {
        request_id: RequestId(1),
        session: None,
        generation: Generation(0),
        result: Ok(CommandOutcome::Completed(CommandResult::EngineInfo {
            info: commands::catalog::types::EngineInfo {
                engine_version: "0.1.0".into(),
                catalog_version: "x".into(),
                protocol_version: "p".into(),
                host: "probe".into(),
                features: vec![],
                plugins: vec![],
                decoders: vec![],
                sessions: vec![],
            },
        })),
    };
    println!("completed: {}", serde_json::to_string(&none_sess).unwrap());
    // 3. Error result.
    let err = commands::CommandError::new(
        commands::error::Stage::Control,
        commands::error::ErrorCode::PermissionDenied,
        "probe error",
    );
    let fail: CommandResponse<CommandResult> = CommandResponse {
        request_id: RequestId(2),
        session: None,
        generation: Generation(0),
        result: Err(err),
    };
    println!("error: {}", serde_json::to_string(&fail).unwrap());
    // 4. Request flatten: unknown field tolerated?
    let parsed: Result<WireRequest, _> =
        serde_json::from_str(r#"{"requestId":9,"cmd":"playback.play","args":{},"evilField":1}"#);
    match parsed {
        Ok(r) => println!("unknown-field request PARSED (ignored silently): {:?}", r.request_id),
        Err(e) => println!("unknown-field request REJECTED: {e}"),
    }
    // 5. RequestId newtype serializes bare?
    println!("requestId json: {}", serde_json::to_string(&RequestId(5)).unwrap());
}
