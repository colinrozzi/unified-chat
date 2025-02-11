mod bindings;

use bindings::exports::ntwk::theater::actor::Guest as ActorGuest;
use bindings::exports::ntwk::theater::http_server::Guest as HttpGuest;
use bindings::exports::ntwk::theater::http_server::{
    HttpRequest as ServerHttpRequest, HttpResponse,
};
use bindings::exports::ntwk::theater::message_server_client::Guest as MessageServerClientGuest;
use bindings::exports::ntwk::theater::websocket_server::Guest as WebSocketGuest;
use bindings::exports::ntwk::theater::websocket_server::{
    MessageType, WebsocketMessage, WebsocketResponse,
};
use bindings::ntwk::theater::filesystem::{create_dir, path_exists, read_file, write_file};
use bindings::ntwk::theater::http_client::{send_http, HttpRequest};
use bindings::ntwk::theater::runtime::log;
use bindings::ntwk::theater::types::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashMap;

// Message types from chat-state
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Message {
    role: String,
    content: String,
    parent: Option<String>,
    id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Chat {
    title: String,
    head: Option<String>,
}

// Anthropic specific types
#[derive(Serialize, Deserialize, Debug, Clone)]
struct AnthropicMessage {
    role: String,
    content: String,
}

// Combined state that includes all necessary data
#[derive(Serialize, Deserialize, Debug, Clone)]
struct State {
    // Chat state
    chat_directory: String,
    // LLM Gateway state
    api_key: String,
    // Any additional state needed for UI
    connected_clients: HashMap<String, bool>,
}

impl Message {
    fn new(role: String, content: String, parent: Option<String>) -> Self {
        let temp_msg = Self {
            role,
            content,
            parent,
            id: String::new(),
        };

        let mut hasher = Sha1::new();
        let temp_json = serde_json::to_string(&temp_msg).unwrap();
        hasher.update(temp_json.as_bytes());
        let id = format!("{:x}", hasher.finalize());

        Self { id, ..temp_msg }
    }
}

// File system operations
impl State {
    fn save_message(&self, msg: &Message) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("data/{}/{}.json", self.chat_directory, msg.id);
        let content = serde_json::to_string(&msg)?;
        write_file(&path, &content).unwrap();
        Ok(())
    }

    fn load_message(&self, id: &str) -> Result<Message, Box<dyn std::error::Error>> {
        let path = format!("data/{}/{}.json", self.chat_directory, id);
        let content = read_file(&path).unwrap();
        let msg: Message = serde_json::from_slice(&content)?;
        Ok(msg)
    }

    fn save_chat(&self, chat: &Chat) -> Result<(), Box<dyn std::error::Error>> {
        let path = format!("data/{}/{}.json", self.chat_directory, chat.title);
        let content = serde_json::to_string(&chat)?;
        write_file(&path, &content).unwrap();

        let path = format!("data/{}/chats.txt", self.chat_directory);
        let chats = read_file(&path).unwrap();
        let mut chats: Vec<String> = serde_json::from_slice(&chats).unwrap_or(Vec::new());
        if !chats.contains(&chat.title) {
            chats.push(chat.title.clone());
            write_file(&path, &serde_json::to_string(&chats)?).unwrap();
        }
        Ok(())
    }

    fn list_chats(&self) -> Result<Vec<Chat>, Box<dyn std::error::Error>> {
        let path = format!("data/{}/chats.txt", self.chat_directory);
        let chats = read_file(&path).unwrap();
        let chats: Vec<String> = serde_json::from_slice(&chats)?;
        let mut chat_list = Vec::new();
        for chat in chats {
            if let Ok(chat) = self.read_chat(&chat) {
                chat_list.push(chat);
            }
        }
        Ok(chat_list)
    }

    fn read_chat(&self, title: &str) -> Result<Chat, Box<dyn std::error::Error>> {
        let path = format!("data/{}/{}.json", self.chat_directory, title);
        let content = read_file(&path).unwrap();
        let chat: Chat = serde_json::from_slice(&content)?;
        Ok(chat)
    }

    fn get_message_history(
        &self,
        message_id: &str,
    ) -> Result<Vec<Message>, Box<dyn std::error::Error>> {
        let mut messages = Vec::new();
        let mut current_id = Some(message_id.to_string());

        while let Some(id) = current_id {
            let msg = self.load_message(&id)?;
            messages.push(msg.clone());
            current_id = msg.parent.clone();
        }

        messages.reverse(); // Oldest first
        Ok(messages)
    }

    // LLM Gateway functionality
    fn generate_response(
        &self,
        messages: Vec<Message>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let anthropic_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            })
            .collect();

        let request = HttpRequest {
            method: "POST".to_string(),
            uri: "https://api.anthropic.com/v1/messages".to_string(),
            headers: vec![
                ("Content-Type".to_string(), "application/json".to_string()),
                ("x-api-key".to_string(), self.api_key.clone()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ],
            body: Some(
                serde_json::to_vec(&json!({
                    "model": "claude-3-5-sonnet-20241022",
                    "max_tokens": 1024,
                    "messages": anthropic_messages,
                }))
                .unwrap(),
            ),
        };

        let http_response = send_http(&request);

        if let Some(body) = http_response.body {
            if let Ok(response_data) = serde_json::from_slice::<Value>(&body) {
                if let Some(text) = response_data["content"][0]["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
        }

        Err("Failed to generate response".into())
    }

    // Create necessary directories
    fn ensure_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create main chat directory
        let base_path = format!("data/{}", self.chat_directory);
        create_dir(&base_path)?;

        // Initialize chats.txt if it doesn't exist
        let chats_path = format!("{}/chats.txt", base_path);
        if !path_exists(&chats_path).expect("Failed to check path existence") {
            write_file(&chats_path, &serde_json::to_string(&Vec::<String>::new())?).unwrap();
        }

        Ok(())
    }
}

struct Component;

impl ActorGuest for Component {
    fn init() -> Vec<u8> {
        log("Initializing unified chat actor");

        // Read API key
        let api_key = read_file("api-key.txt").unwrap();
        let api_key = String::from_utf8(api_key).unwrap().trim().to_string();

        let initial_state = State {
            chat_directory: "chats".to_string(),
            api_key,
            connected_clients: HashMap::new(),
        };

        // Ensure directories exist
        initial_state.ensure_directories().unwrap();

        serde_json::to_vec(&initial_state).unwrap()
    }
}

impl HttpGuest for Component {
    fn handle_request(req: ServerHttpRequest, state: Json) -> (HttpResponse, Json) {
        log(&format!("Handling HTTP request for: {}", req.uri));

        match (req.method.as_str(), req.uri.as_str()) {
            ("GET", "/") | ("GET", "/index.html") => {
                let content = read_file("index.html").unwrap();
                (
                    HttpResponse {
                        status: 200,
                        headers: vec![("Content-Type".to_string(), "text/html".to_string())],
                        body: Some(content),
                    },
                    state,
                )
            }
            ("GET", "/styles.css") => {
                let content = read_file("styles.css").unwrap();
                (
                    HttpResponse {
                        status: 200,
                        headers: vec![("Content-Type".to_string(), "text/css".to_string())],
                        body: Some(content),
                    },
                    state,
                )
            }
            ("GET", "/chat.js") => {
                let content = read_file("chat.js").unwrap();
                (
                    HttpResponse {
                        status: 200,
                        headers: vec![(
                            "Content-Type".to_string(),
                            "application/javascript".to_string(),
                        )],
                        body: Some(content),
                    },
                    state,
                )
            }

            // API endpoints
            ("GET", "/api/chats") => {
                let current_state: State = serde_json::from_slice(&state).unwrap();
                match current_state.list_chats() {
                    Ok(chats) => (
                        HttpResponse {
                            status: 200,
                            headers: vec![(
                                "Content-Type".to_string(),
                                "application/json".to_string(),
                            )],
                            body: Some(
                                serde_json::to_vec(&json!({
                                    "status": "success",
                                    "chats": chats
                                }))
                                .unwrap(),
                            ),
                        },
                        state,
                    ),
                    Err(_) => (
                        HttpResponse {
                            status: 500,
                            headers: vec![],
                            body: Some(b"Failed to list chats".to_vec()),
                        },
                        state,
                    ),
                }
            }

            ("POST", "/api/chats") => {
                let current_state: State = serde_json::from_slice(&state).unwrap();

                if let Some(body) = req.body {
                    if let Ok(data) = serde_json::from_slice::<Value>(&body) {
                        if let Some(title) = data["title"].as_str() {
                            let chat = Chat {
                                title: title.to_string(),
                                head: None,
                            };

                            if current_state.save_chat(&chat).is_ok() {
                                return (
                                    HttpResponse {
                                        status: 200,
                                        headers: vec![(
                                            "Content-Type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body: Some(
                                            serde_json::to_vec(&json!({
                                                "status": "success",
                                                "chat": chat
                                            }))
                                            .unwrap(),
                                        ),
                                    },
                                    state,
                                );
                            }
                        }
                    }
                }

                (
                    HttpResponse {
                        status: 400,
                        headers: vec![],
                        body: Some(b"Invalid request".to_vec()),
                    },
                    state,
                )
            }

            ("GET", path) if path.starts_with("/api/chats/") => {
                let current_state: State = serde_json::from_slice(&state).unwrap();
                let chat_id = path.trim_start_matches("/api/chats/");

                match current_state.read_chat(chat_id) {
                    Ok(chat) => {
                        if let Some(head) = &chat.head {
                            if let Ok(messages) = current_state.get_message_history(head) {
                                return (
                                    HttpResponse {
                                        status: 200,
                                        headers: vec![(
                                            "Content-Type".to_string(),
                                            "application/json".to_string(),
                                        )],
                                        body: Some(
                                            serde_json::to_vec(&json!({
                                                "status": "success",
                                                "chat": chat,
                                                "messages": messages
                                            }))
                                            .unwrap(),
                                        ),
                                    },
                                    state,
                                );
                            }
                        }
                        (
                            HttpResponse {
                                status: 404,
                                headers: vec![],
                                body: Some(b"Chat messages not found".to_vec()),
                            },
                            state,
                        )
                    }
                    Err(_) => (
                        HttpResponse {
                            status: 404,
                            headers: vec![],
                            body: Some(b"Chat not found".to_vec()),
                        },
                        state,
                    ),
                }
            }

            // Default 404 response
            _ => (
                HttpResponse {
                    status: 404,
                    headers: vec![],
                    body: Some(b"Not Found".to_vec()),
                },
                state,
            ),
        }
    }
}

// Handle WebSocket messages for real-time updates
impl WebSocketGuest for Component {
    fn handle_message(msg: WebsocketMessage, state: Json) -> (Json, WebsocketResponse) {
        let current_state: State = serde_json::from_slice(&state).unwrap();

        match msg.ty {
            MessageType::Text => {
                if let Some(text) = msg.text {
                    if let Ok(command) = serde_json::from_str::<Value>(&text) {
                        match command["type"].as_str() {
                            Some("new_chat") => {
                                if let Some(title) = command["title"].as_str() {
                                    // Create new chat
                                    let chat = Chat {
                                        title: title.to_string(),
                                        head: None,
                                    };

                                    // Save the chat
                                    if current_state.save_chat(&chat).is_ok() {
                                        // Get updated chat list
                                        if let Ok(chats) = current_state.list_chats() {
                                            log(&format!("Created new chat: {}", title));
                                            // Send success response with updated chat list
                                            return (
                                                serde_json::to_vec(&current_state).unwrap(),
                                                WebsocketResponse {
                                                    messages: vec![WebsocketMessage {
                                                        ty: MessageType::Text,
                                                        text: Some(
                                                            serde_json::json!({
                                                                "status": "success",
                                                                "type": "chat_update",
                                                                "chats": chats
                                                            })
                                                            .to_string(),
                                                        ),
                                                        data: None,
                                                    }],
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                            Some("send_message") => {
                                if let (Some(content), Some(chat_id)) =
                                    (command["content"].as_str(), command["chat_id"].as_str())
                                {
                                    // Create user message
                                    let user_msg =
                                        Message::new("user".to_string(), content.to_string(), None);

                                    if current_state.save_message(&user_msg).is_ok() {
                                        // Update chat head
                                        if let Ok(mut chat) = current_state.read_chat(chat_id) {
                                            chat.head = Some(user_msg.id.clone());
                                            current_state.save_chat(&chat).ok();

                                            // Get message history for context
                                            if let Ok(messages) =
                                                current_state.get_message_history(&user_msg.id)
                                            {
                                                // Generate AI response
                                                if let Ok(ai_response) =
                                                    current_state.generate_response(messages)
                                                {
                                                    let ai_msg = Message::new(
                                                        "assistant".to_string(),
                                                        ai_response,
                                                        Some(user_msg.id.clone()),
                                                    );

                                                    if current_state.save_message(&ai_msg).is_ok() {
                                                        chat.head = Some(ai_msg.id.clone());
                                                        current_state.save_chat(&chat).ok();

                                                        // Broadcast update to all connected clients
                                                        return (
                                                            serde_json::to_vec(&current_state).unwrap(),
                                                            WebsocketResponse {
                                                                messages: vec![WebsocketMessage {
                                                                    ty: MessageType::Text,
                                                                    text: Some(serde_json::json!({
                                                                        "type": "message_update",
                                                                        "chat_id": chat_id,
                                                                        "messages": [user_msg, ai_msg]
                                                                    }).to_string()),
                                                                    data: None,
                                                                }]
                                                            }
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Some("get_all") => {
                                log("Handling get_all command");
                                match current_state.list_chats() {
                                    Ok(chats) => {
                                        log(&format!("Successfully listed {} chats", chats.len()));
                                        return (
                                            serde_json::to_vec(&current_state).unwrap(),
                                            WebsocketResponse {
                                                messages: vec![WebsocketMessage {
                                                    ty: MessageType::Text,
                                                    text: Some(
                                                        serde_json::json!({
                                                            "status": "success",
                                                            "type": "chat_update",
                                                            "chats": chats
                                                        })
                                                        .to_string(),
                                                    ),
                                                    data: None,
                                                }],
                                            },
                                        );
                                    }
                                    Err(e) => {
                                        log(&format!("Error listing chats: {}", e));
                                    }
                                }
                            }
                            _ => {
                                log("Unknown command type received");
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        (
            serde_json::to_vec(&current_state).unwrap(),
            WebsocketResponse { messages: vec![] },
        )
    }
}

impl MessageServerClientGuest for Component {
    fn handle_send(msg: Vec<u8>, state: Json) -> Json {
        log("Handling message server client send");
        let msg_str = String::from_utf8(msg).unwrap();
        log(&msg_str);
        state
    }

    fn handle_request(msg: Vec<u8>, state: Json) -> (Vec<u8>, Json) {
        log("Handling message server client request");
        let msg_str = String::from_utf8(msg).unwrap();
        log(&msg_str);
        (vec![], state)
    }
}

bindings::export!(Component with_types_in bindings);
