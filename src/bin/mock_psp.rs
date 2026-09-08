// Mock PSP for the invoice service to call over HTTP in place of a real processor.
use rand::Rng;
use serde::Deserialize;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

#[derive(Deserialize)]
struct ChargeRequest {
    card_token: String,
}

struct ParsedRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(9090);

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("mock-psp listening on 0.0.0.0:{port}");

    loop {
        let (stream, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream).await {
                eprintln!("mock-psp: connection error: {e}");
            }
        });
    }
}

async fn handle_connection(stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);

    let request = match read_request(&mut reader).await? {
        Some(r) => r,
        None => return Ok(()), 
    };

    let mut stream = reader.into_inner();

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => write_json(&mut stream, 200, br#"{"status":"ok"}"#).await,
        ("POST", "/charges") => handle_charge(&mut stream, &request.body).await,
        _ => write_json(&mut stream, 404, br#"{"error":"not_found"}"#).await,
    }
}

async fn read_request(
    reader: &mut BufReader<TcpStream>,
) -> std::io::Result<Option<ParsedRequest>> {
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).await? == 0 {
        return Ok(None);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();

    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                content_length = value.trim().parse().unwrap_or(0);
            }
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }

    Ok(Some(ParsedRequest { method, path, body }))
}

async fn handle_charge(stream: &mut TcpStream, body: &[u8]) -> std::io::Result<()> {
    let charge: Option<ChargeRequest> = serde_json::from_slice(body).ok();

    let card_token = match &charge {
        Some(c) => c.card_token.as_str(),
        None => return write_json(stream, 400, br#"{"error":"invalid_request_body"}"#).await,
    };

    println!("mock-psp: charge request card_token={card_token}");

    match card_token {
        "tok_success" => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let body = serde_json::json!({ "status": "succeeded", "psp_ref": Uuid::new_v4() });
            write_json(stream, 200, body.to_string().as_bytes()).await
        }
        "tok_insufficient_funds" => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            write_json(
                stream,
                200,
                br#"{"status":"failed","code":"insufficient_funds"}"#,
            )
            .await
        }
        "tok_card_declined" => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            write_json(stream, 200, br#"{"status":"failed","code":"card_declined"}"#).await
        }
        "tok_timeout" => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let body = serde_json::json!({ "status": "succeeded", "psp_ref": Uuid::new_v4() });
            write_json(stream, 200, body.to_string().as_bytes()).await
        }
        "tok_network_error" => {
            if rand::thread_rng().gen_bool(0.5) {
                write_json(stream, 500, br#"{"error":"internal_error"}"#).await
            } else {
                println!("mock-psp: simulating dropped connection for tok_network_error");
                stream.shutdown().await
            }
        }
        _ => write_json(stream, 400, br#"{"error":"unknown_card_token"}"#).await,
    }
}

async fn write_json(stream: &mut TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.shutdown().await
}
