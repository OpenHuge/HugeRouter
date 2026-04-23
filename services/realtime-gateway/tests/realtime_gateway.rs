use futures_util::{SinkExt, StreamExt};
use http::StatusCode;
use protocol_realtime::{
    AllowedSessionParams, ClientEvent, EphemeralSessionTokenClaims, EphemeralTokenSigner,
    REALTIME_CONNECT_SCOPE, RealtimeTransport, ResponseCreateInput,
};
use realtime_gateway::{RealtimeGatewayConfig, UpstreamMode, serve};
use serde_json::Value;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, client::IntoClientRequest, protocol::Message},
};

struct TestServer {
    addr: std::net::SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start(upstream_mode: UpstreamMode, idle_timeout: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener should bind");
        let addr = listener
            .local_addr()
            .expect("listener should have local addr");
        let config = RealtimeGatewayConfig {
            bind_addr: addr,
            signing_secret: "test-secret".to_string(),
            idle_timeout,
            session_duration_cap: Duration::from_secs(300),
            upstream_mode,
            stub_response_text: "stub-response".to_string(),
            uses_insecure_default_secret: false,
        };

        let handle = tokio::spawn(async move {
            serve(listener, config).await.expect("server should run");
        });

        for _ in 0..20 {
            if TcpStream::connect(addr).await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Self { addr, handle }
    }

    fn ws_url(&self, path: &str) -> String {
        format!("ws://{}{}", self.addr, path)
    }

    fn issue_token(&self, models: Vec<&str>, scopes: Vec<&str>, expires_in_seconds: i64) -> String {
        let signer = EphemeralTokenSigner::new("test-secret".as_bytes().to_vec())
            .expect("signer should initialize");
        let claims = EphemeralSessionTokenClaims {
            subject: "browser-client".to_string(),
            tenant_id: "tenant-1".to_string(),
            project_id: "project-1".to_string(),
            scopes: scopes.into_iter().map(ToString::to_string).collect(),
            transport: RealtimeTransport::Websocket,
            expires_at: chrono::Utc::now().timestamp() + expires_in_seconds,
            not_before: None,
            allowed_session_params: AllowedSessionParams {
                model_aliases: models.into_iter().map(ToString::to_string).collect(),
                max_duration_seconds: 120,
                max_concurrency: 1,
            },
            token_id: Some("tok_test".to_string()),
        };

        signer.sign(&claims).expect("token should sign")
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

#[tokio::test]
async fn rejects_missing_authentication() {
    let server = TestServer::start(UpstreamMode::Echo, Duration::from_secs(30)).await;
    let request = server
        .ws_url("/v1/realtime?model=realtime-default")
        .into_client_request()
        .expect("ws request should build");
    let error = connect_async(request)
        .await
        .expect_err("handshake should be rejected");

    assert_handshake_status(error, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn accepts_valid_handshake_and_echoes_response() {
    let server = TestServer::start(UpstreamMode::Echo, Duration::from_secs(30)).await;
    let token = server.issue_token(vec!["realtime-default"], vec![REALTIME_CONNECT_SCOPE], 300);
    let mut request = server
        .ws_url("/v1/realtime?model=realtime-default")
        .into_client_request()
        .expect("ws request should build");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {token}")
            .parse()
            .expect("auth header should parse"),
    );

    let (mut stream, _) = connect_async(request)
        .await
        .expect("handshake should succeed");

    let created = next_text_message(&mut stream).await;
    assert_eq!(created["type"], "session.created");

    let payload = serde_json::to_string(&ClientEvent::ResponseCreate {
        response: ResponseCreateInput {
            input_text: Some("hello".to_string()),
            instructions: None,
        },
    })
    .expect("client event should serialize");
    stream
        .send(Message::Text(payload.into()))
        .await
        .expect("client should send event");

    let response_created = next_text_message(&mut stream).await;
    assert_eq!(response_created["type"], "response.created");
    let delta = next_text_message(&mut stream).await;
    assert_eq!(delta["type"], "response.output_text.delta");
    assert_eq!(delta["delta"], "echo:realtime-default:hello");
    let completed = next_text_message(&mut stream).await;
    assert_eq!(completed["type"], "response.completed");
}

#[tokio::test]
async fn rejects_invalid_model_parameter() {
    let server = TestServer::start(UpstreamMode::Echo, Duration::from_secs(30)).await;
    let token = issue_test_token(vec!["realtime-default"], vec![REALTIME_CONNECT_SCOPE], 300);
    let mut request = server
        .ws_url("/v1/realtime?model=not-allowed")
        .into_client_request()
        .expect("ws request should build");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {token}")
            .parse()
            .expect("auth header should parse"),
    );
    let error = connect_async(request)
        .await
        .expect_err("handshake should be rejected");

    assert_handshake_status(error, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn closes_idle_session_after_timeout() {
    let server = TestServer::start(UpstreamMode::Echo, Duration::from_millis(150)).await;
    let token = server.issue_token(vec!["realtime-default"], vec![REALTIME_CONNECT_SCOPE], 300);
    let mut request = server
        .ws_url("/v1/realtime?model=realtime-default")
        .into_client_request()
        .expect("ws request should build");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {token}")
            .parse()
            .expect("auth header should parse"),
    );

    let (mut stream, _) = connect_async(request)
        .await
        .expect("handshake should succeed");
    let _ = next_text_message(&mut stream).await;

    let frame = stream.next().await.expect("close frame should arrive");
    let frame = frame.expect("close frame should be readable");
    match frame {
        Message::Close(Some(close_frame)) => {
            assert_eq!(u16::from(close_frame.code), 1008);
            assert_eq!(close_frame.reason, "idle timeout");
        }
        other => panic!("expected close frame, got {other:?}"),
    }
}

#[tokio::test]
async fn closes_session_when_upstream_fails() {
    let server = TestServer::start(UpstreamMode::FailAllResponses, Duration::from_secs(30)).await;
    let token = server.issue_token(vec!["realtime-default"], vec![REALTIME_CONNECT_SCOPE], 300);
    let mut request = server
        .ws_url("/v1/realtime?model=realtime-default")
        .into_client_request()
        .expect("ws request should build");
    request.headers_mut().insert(
        "Authorization",
        format!("Bearer {token}")
            .parse()
            .expect("auth header should parse"),
    );

    let (mut stream, _) = connect_async(request)
        .await
        .expect("handshake should succeed");
    let _ = next_text_message(&mut stream).await;

    let payload = serde_json::to_string(&ClientEvent::ResponseCreate {
        response: ResponseCreateInput {
            input_text: Some("hello".to_string()),
            instructions: None,
        },
    })
    .expect("client event should serialize");
    stream
        .send(Message::Text(payload.into()))
        .await
        .expect("client should send event");

    let _ = next_text_message(&mut stream).await;
    let error = next_text_message(&mut stream).await;
    assert_eq!(error["type"], "error");
    assert_eq!(error["error"]["code"], "provider_unavailable");

    let frame = stream.next().await.expect("close frame should arrive");
    let frame = frame.expect("close frame should be readable");
    match frame {
        Message::Close(Some(close_frame)) => {
            assert_eq!(u16::from(close_frame.code), 1011);
            assert_eq!(close_frame.reason, "upstream failure");
        }
        other => panic!("expected close frame, got {other:?}"),
    }
}

async fn next_text_message(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    let message = stream
        .next()
        .await
        .expect("message should arrive")
        .expect("message should be readable");
    match message {
        Message::Text(payload) => {
            serde_json::from_str(payload.as_ref()).expect("payload should be json")
        }
        other => panic!("expected text message, got {other:?}"),
    }
}

fn issue_test_token(models: Vec<&str>, scopes: Vec<&str>, expires_in_seconds: i64) -> String {
    let signer = EphemeralTokenSigner::new("test-secret".as_bytes().to_vec())
        .expect("signer should initialize");
    let claims = EphemeralSessionTokenClaims {
        subject: "browser-client".to_string(),
        tenant_id: "tenant-1".to_string(),
        project_id: "project-1".to_string(),
        scopes: scopes.into_iter().map(ToString::to_string).collect(),
        transport: RealtimeTransport::Websocket,
        expires_at: chrono::Utc::now().timestamp() + expires_in_seconds,
        not_before: None,
        allowed_session_params: AllowedSessionParams {
            model_aliases: models.into_iter().map(ToString::to_string).collect(),
            max_duration_seconds: 120,
            max_concurrency: 1,
        },
        token_id: Some("tok_test".to_string()),
    };

    signer.sign(&claims).expect("token should sign")
}

fn assert_handshake_status(error: WsError, expected: StatusCode) {
    match error {
        WsError::Http(response) => assert_eq!(response.status(), expected),
        other => panic!("expected http handshake rejection, got {other:?}"),
    }
}
