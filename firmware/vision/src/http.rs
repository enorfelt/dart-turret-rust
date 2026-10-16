use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use esp_idf_svc::http::server::ws::EspHttpWsDetachedSender;
use esp_idf_svc::http::server::{Configuration, EspHttpConnection, EspHttpServer, Request};
use esp_idf_svc::http::Method;
use esp_idf_svc::io::Write;
use esp_idf_svc::sys::{MALLOC_CAP_8BIT, MALLOC_CAP_SPIRAM};
use esp_idf_svc::ws::FrameType;
use vision_core::people::Role;
use vision_core::web::{json_string, query_param, resource};

use crate::pipeline::{self, EnrollStatus, Shared};
use crate::storage;
use crate::task;

const INDEX: &str = include_str!("web/index.html");
const MAX_STREAM_CLIENTS: usize = 3;
const PEOPLE: &str = "/api/people/";

type Clients = Arc<Mutex<Vec<EspHttpWsDetachedSender>>>;
type HandlerResult = anyhow::Result<()>;

pub fn start(
    shared: Arc<Shared>,
    requests: Sender<pipeline::Request>,
) -> anyhow::Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(&Configuration {
        stack_size: 10 * 1024,
        uri_match_wildcard: true,
        max_open_sockets: 6,
        task_caps: MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT,
        ..Default::default()
    })?;

    server.fn_handler("/", Method::Get, |request| -> HandlerResult {
        request
            .into_response(200, None, &[("Content-Type", "text/html; charset=utf-8")])?
            .write_all(INDEX.as_bytes())?;
        Ok(())
    })?;

    let state = shared.clone();
    server.fn_handler(
        "/api/people",
        Method::Get,
        move |request| -> HandlerResult { json(request, 200, &people_json(&state)) },
    )?;

    let state = shared.clone();
    let sender = Mutex::new(requests.clone());
    server.fn_handler(
        "/api/people",
        Method::Post,
        move |request| -> HandlerResult {
            let name = query_param(request.uri(), "name").unwrap_or_default();
            let role = match query_param(request.uri(), "role").as_deref() {
                Some("target") => Role::Target,
                Some("protected") => Role::Protected,
                _ => {
                    return json(
                        request,
                        400,
                        r#"{"error":"role must be target or protected"}"#,
                    )
                }
            };
            submit(
                request,
                &state,
                &sender,
                pipeline::Request::Enroll { name, role },
            )
        },
    )?;

    let state = shared.clone();
    let sender = Mutex::new(requests.clone());
    server.fn_handler(
        "/api/people/*",
        Method::Post,
        move |request| -> HandlerResult {
            match resource(request.uri(), PEOPLE) {
                Some((id, "samples")) => submit(
                    request,
                    &state,
                    &sender,
                    pipeline::Request::AddSample { id },
                ),
                _ => json(request, 404, r#"{"error":"not found"}"#),
            }
        },
    )?;

    let sender = Mutex::new(requests);
    server.fn_handler(
        "/api/people/*",
        Method::Delete,
        move |request| -> HandlerResult {
            let Some((id, "")) = resource(request.uri(), PEOPLE) else {
                return json(request, 404, r#"{"error":"not found"}"#);
            };
            sender
                .lock()
                .unwrap()
                .send(pipeline::Request::Remove { id })?;
            request.into_status_response(204)?;
            Ok(())
        },
    )?;

    server.fn_handler("/api/people/*", Method::Get, |request| -> HandlerResult {
        let photo = match resource(request.uri(), PEOPLE) {
            Some((id, "photo")) => storage::load_snapshot(id),
            _ => None,
        };
        match photo {
            Some(jpeg) => {
                request
                    .into_response(
                        200,
                        None,
                        &[
                            ("Content-Type", "image/jpeg"),
                            ("Cache-Control", "no-store"),
                        ],
                    )?
                    .write_all(&jpeg)?;
                Ok(())
            }
            None => json(request, 404, r#"{"error":"not found"}"#),
        }
    })?;

    let state = shared.clone();
    server.fn_handler(
        "/api/enrollment",
        Method::Get,
        move |request| -> HandlerResult {
            let status = state.enroll.lock().unwrap().clone();
            let mut body = String::from(r#"{"state":"#);
            match status {
                EnrollStatus::Idle => body.push_str(r#""idle"}"#),
                EnrollStatus::Running => body.push_str(r#""running"}"#),
                EnrollStatus::Done { id } => body.push_str(&format!(r#""done","id":{id}}}"#)),
                EnrollStatus::Failed { reason } => {
                    body.push_str(r#""failed","reason":"#);
                    json_string(&reason, &mut body);
                    body.push('}');
                }
            }
            json(request, 200, &body)
        },
    )?;

    let clients: Clients = Arc::default();
    let connected = clients.clone();
    let state = shared.clone();
    // ESP-IDF 5.5 answers the handshake without calling the handler, so a client is registered when its
    // first message arrives; clients send one right after opening.
    server.ws_handler("/stream", None, move |ws| {
        if ws.is_closed() {
            let mut clients = connected.lock().unwrap();
            let before = clients.len();
            clients.retain(|client| client.session() != ws.session() && !client.is_closed());
            state.set_viewers(clients.len());
            if clients.len() != before {
                log::info!(
                    "stream client {} closed, {} viewers",
                    ws.session(),
                    clients.len()
                );
            }
            return Ok(());
        }
        if !ws.is_new() {
            let mut discard = [0u8; 64];
            ws.recv(&mut discard)?;
        }
        let mut clients = connected.lock().unwrap();
        clients.retain(|client| !client.is_closed());
        if clients
            .iter()
            .any(|client| client.session() == ws.session())
        {
            return Ok(());
        }
        if clients.len() >= MAX_STREAM_CLIENTS {
            log::warn!(
                "stream client {} refused, {} already connected",
                ws.session(),
                clients.len()
            );
            return ws.send(FrameType::Close, &[]);
        }
        clients.push(ws.create_detached_sender()?);
        state.set_viewers(clients.len());
        log::info!(
            "stream client {} connected, {} viewers",
            ws.session(),
            clients.len()
        );
        Ok::<(), esp_idf_svc::sys::EspError>(())
    })?;

    task::spawn_with_psram_stack(c"stream", 6 * 1024, move || stream(shared, clients))?;

    Ok(server)
}

fn stream(shared: Arc<Shared>, clients: Clients) {
    let mut seq = 0;
    loop {
        let Some((next, frame)) = shared.next_frame(seq, Duration::from_secs(1)) else {
            let mut clients = clients.lock().unwrap();
            clients.retain(|client| !client.is_closed());
            shared.set_viewers(clients.len());
            continue;
        };
        seq = next;
        let mut senders: Vec<EspHttpWsDetachedSender> = clients.lock().unwrap().clone();
        let mut dropped = Vec::new();
        for sender in senders.iter_mut() {
            if sender.is_closed() {
                dropped.push(sender.session());
            } else if let Err(error) = sender.send(FrameType::Binary(false), &frame) {
                log::warn!("stream client {}: send failed: {error}", sender.session());
                dropped.push(sender.session());
            }
        }
        if !dropped.is_empty() {
            let mut clients = clients.lock().unwrap();
            clients.retain(|client| !dropped.contains(&client.session()));
            shared.set_viewers(clients.len());
        }
    }
}

fn submit(
    request: Request<&mut EspHttpConnection>,
    state: &Shared,
    sender: &Mutex<Sender<pipeline::Request>>,
    work: pipeline::Request,
) -> HandlerResult {
    if !state.card {
        return json(request, 503, r#"{"error":"no SD card"}"#);
    }
    {
        let mut status = state.enroll.lock().unwrap();
        if *status == EnrollStatus::Running {
            return json(
                request,
                409,
                r#"{"error":"an enrollment is already running"}"#,
            );
        }
        *status = EnrollStatus::Running;
    }
    sender.lock().unwrap().send(work)?;
    json(request, 202, r#"{"state":"running"}"#)
}

fn people_json(state: &Shared) -> String {
    let people = state.people.lock().unwrap().clone();
    let mut body = format!(r#"{{"card":{},"people":["#, state.card);
    for (i, person) in people.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(r#"{{"id":{},"name":"#, person.id));
        json_string(&person.name, &mut body);
        let role = match person.role {
            Role::Target => "target",
            Role::Protected => "protected",
        };
        body.push_str(&format!(
            r#","role":"{role}","samples":{}}}"#,
            person.samples
        ));
    }
    body.push_str("]}");
    body
}

fn json(request: Request<&mut EspHttpConnection>, status: u16, body: &str) -> HandlerResult {
    request
        .into_response(
            status,
            None,
            &[
                ("Content-Type", "application/json"),
                ("Cache-Control", "no-store"),
            ],
        )?
        .write_all(body.as_bytes())?;
    Ok(())
}
