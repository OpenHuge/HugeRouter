use anyhow::Result;
use protocol_realtime::{
    AllowedSessionParams, EphemeralSessionTokenClaims, EphemeralTokenSigner,
    REALTIME_CONNECT_SCOPE, RealtimeTransport,
};

fn parse_duration(argument: Option<&String>) -> u64 {
    argument
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(900)
}

fn parse_models(argument: Option<&String>) -> Vec<String> {
    argument
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|models| !models.is_empty())
        .unwrap_or_else(|| vec!["realtime-default".to_string()])
}

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 5 {
        eprintln!(
            "usage: cargo run -p protocol-realtime --example issue_token -- <secret> <subject> <tenant_id> <project_id> [models_csv] [duration_seconds]"
        );
        std::process::exit(1);
    }

    let signer = EphemeralTokenSigner::new(args[1].as_bytes().to_vec())?;
    let duration_seconds = parse_duration(args.get(6));
    let claims = EphemeralSessionTokenClaims {
        subject: args[2].clone(),
        tenant_id: args[3].clone(),
        project_id: args[4].clone(),
        scopes: vec![REALTIME_CONNECT_SCOPE.to_string()],
        transport: RealtimeTransport::Websocket,
        expires_at: chrono::Utc::now().timestamp() + i64::try_from(duration_seconds)?,
        not_before: None,
        allowed_session_params: AllowedSessionParams {
            model_aliases: parse_models(args.get(5)),
            max_duration_seconds: duration_seconds,
            max_concurrency: 1,
        },
        token_id: None,
    };

    println!("{}", signer.sign(&claims)?);
    Ok(())
}
