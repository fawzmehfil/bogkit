use crate::{
    game::{EngineCommand, EngineHandle},
    model::{GameSnapshot, Skin},
};
use axum::{
    Router,
    body::Body,
    extract::{
        Path, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use qrcode::{QrCode, render::svg};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone)]
struct AppState {
    engine: EngineHandle,
    dist: Arc<PathBuf>,
    next_connection_id: Arc<AtomicU64>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Register {
        player_id: String,
        name: String,
        skin: Skin,
    },
    SelectSkin {
        player_id: String,
        skin: Skin,
    },
    Input {
        player_id: String,
        sequence: u64,
        axis_x: f32,
        axis_y: f32,
    },
    StartRun,
    ChooseUpgrade {
        player_id: String,
        draft_id: u64,
        choice_id: u64,
    },
    EquipRune {
        player_id: String,
        drop_id: u64,
    },
    SalvageRune {
        player_id: String,
        drop_id: u64,
    },
    Rematch,
}

pub async fn serve(port: u16, engine: EngineHandle) {
    let state = AppState {
        engine,
        dist: Arc::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/dist")),
        next_connection_id: Arc::new(AtomicU64::new(1)),
    };
    let app = Router::new()
        .route("/ws", get(ws_upgrade))
        .route("/qr.svg", get(qr))
        .route("/health", get(health))
        .route("/assets/{*path}", get(asset))
        .route("/{*path}", get(index))
        .route("/", get(index))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind Bogbound server");
    println!("BOGBOUND is ready: http://localhost:{port}");
    println!("Open the LAN URL shown in the lobby on every phone or laptop.");
    axum::serve(listener, app).await.expect("serve Bogbound");
}

async fn ws_upgrade(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    let id = state.next_connection_id.fetch_add(1, Ordering::Relaxed);
    ws.on_upgrade(move |socket| handle_socket(socket, state.engine, id))
}

fn personalize(mut snapshot: GameSnapshot, player_id: Option<&str>) -> String {
    if let Some(id) = player_id {
        snapshot.you = Some(id.into());
        snapshot.upgrade_draft = snapshot
            .private_upgrades
            .iter()
            .find(|(pid, _)| pid == id)
            .map(|(_, d)| d.clone());
        snapshot.rune_draft = snapshot
            .private_runes
            .iter()
            .find(|(pid, _)| pid == id)
            .map(|(_, d)| d.clone());
        snapshot
            .pickups
            .retain(|p| p.owner_id.as_deref().is_none_or(|owner| owner == id));
    }
    serde_json::to_string(&snapshot).expect("serialize game snapshot")
}

async fn handle_socket(mut socket: WebSocket, engine: EngineHandle, connection_id: u64) {
    if engine
        .commands
        .send(EngineCommand::SocketOpened { connection_id })
        .is_err()
    {
        return;
    }
    let mut snapshots = engine.snapshots.clone();
    let mut player_id: Option<String> = None;
    let hello = personalize(snapshots.borrow_and_update().clone(), None);
    if socket.send(Message::Text(hello.into())).await.is_err() {
        let _ = engine
            .commands
            .send(EngineCommand::SocketClosed { connection_id });
        return;
    }
    loop {
        tokio::select! {changed=snapshots.changed()=>{if changed.is_err(){break}let json=personalize(snapshots.borrow_and_update().clone(),player_id.as_deref());if socket.send(Message::Text(json.into())).await.is_err(){break}},incoming=socket.recv()=>{let Some(Ok(Message::Text(text)))=incoming else{break};let Ok(message)=serde_json::from_str::<ClientMessage>(&text)else{continue};let command=match message{ClientMessage::Register{player_id:id,name,skin}=>{player_id=Some(id.clone());EngineCommand::Register{connection_id,player_id:id,name,skin}},ClientMessage::SelectSkin{player_id,skin}=>EngineCommand::SelectSkin{player_id,skin},ClientMessage::Input{player_id,sequence,axis_x,axis_y}=>EngineCommand::Input{connection_id,player_id,sequence,x:axis_x,y:axis_y},ClientMessage::StartRun=>EngineCommand::StartRun,ClientMessage::ChooseUpgrade{player_id,draft_id,choice_id}=>EngineCommand::ChooseUpgrade{player_id,draft_id,choice_id},ClientMessage::EquipRune{player_id,drop_id}=>EngineCommand::EquipRune{player_id,drop_id},ClientMessage::SalvageRune{player_id,drop_id}=>EngineCommand::SalvageRune{player_id,drop_id},ClientMessage::Rematch=>EngineCommand::Rematch};if engine.commands.send(command).is_err(){break}}}
    }
    let _ = engine
        .commands
        .send(EngineCommand::SocketClosed { connection_id });
}

async fn index(State(state): State<AppState>) -> Response {
    file_response(state.dist.join("index.html"), "text/html; charset=utf-8")
}
async fn asset(State(state): State<AppState>, Path(path): Path<String>) -> Response {
    if path.contains("..") || path.starts_with('/') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let content_type = if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".woff2") {
        "font/woff2"
    } else if path.ends_with(".woff") {
        "font/woff"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "application/octet-stream"
    };
    file_response(state.dist.join("assets").join(path), content_type)
}
fn file_response(path: PathBuf, content_type: &'static str) -> Response {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut response = Response::new(Body::from(bytes));
            response
                .headers_mut()
                .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
            response
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Frontend missing. Run npm install && npm run build in examples/bogbound/web.",
        )
            .into_response(),
    }
}
async fn qr(State(state): State<AppState>) -> Response {
    let url = state.engine.snapshots.borrow().join_url.clone();
    let body = QrCode::new(url.as_bytes())
        .expect("QR")
        .render::<svg::Color>()
        .min_dimensions(260, 260)
        .dark_color(svg::Color("#101b18"))
        .light_color(svg::Color("#f7e6b5"))
        .build();
    let mut response = Response::new(Body::from(body));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("image/svg+xml; charset=utf-8"),
    );
    response
}
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let s = state.engine.snapshots.borrow();
    axum::Json(
        serde_json::json!({"ok":true,"phase":s.phase,"players":s.connected_players,"tick":s.tick,"fold_commits":s.metrics.fold_commits}),
    )
}
