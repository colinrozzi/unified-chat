// State management
let currentChatTitle = null;
let currentMessageParentId = null;
let messageCache = new Map();
let ws = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 5;

// UI Elements
const messageInput = document.getElementById('messageInput');
const messageArea = document.getElementById('messageArea');

// Auto-resize textarea
function adjustTextareaHeight() {
    messageInput.style.height = 'auto';
    messageInput.style.height = Math.min(messageInput.scrollHeight, 200) + 'px';
}

messageInput.addEventListener('input', adjustTextareaHeight);

// WebSocket connection management
function updateConnectionStatus(status) {
    const statusElement = document.getElementById('connectionStatus');
    statusElement.className = 'connection-status ' + status;
    
    switch(status) {
        case 'connected':
            statusElement.textContent = 'Connected';
            break;
        case 'disconnected':
            statusElement.textContent = 'Disconnected';
            break;
        case 'connecting':
            statusElement.textContent = 'Connecting...';
            break;
    }
}

function connectWebSocket() {
    updateConnectionStatus('connecting');
    
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    //const wsUrl = `${protocol}//${window.location.host}/`;
    const wsUrl = 'ws://localhost:8082/';
    
    ws = new WebSocket(wsUrl);
    
    ws.onopen = () => {
        console.log('WebSocket connected');
        updateConnectionStatus('connected');
        reconnectAttempts = 0;
        // Request initial data
        sendWebSocketMessage({
            type: 'get_all'
        });
    };
    
    ws.onclose = () => {
        console.log('WebSocket disconnected');
        updateConnectionStatus('disconnected');
        if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
            reconnectAttempts++;
            setTimeout(connectWebSocket, 1000 * Math.min(reconnectAttempts, 30));
        }
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateConnectionStatus('disconnected');
    };
    
    ws.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleWebSocketMessage(data);
        } catch (error) {
            console.error('Error parsing WebSocket message:', error);
        }
    };
}

function sendWebSocketMessage(message) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(message));
    } else {
        console.warn('WebSocket not connected');
        updateConnectionStatus('disconnected');
    }
}

function handleWebSocketMessage(data) {
    if (data.status === 'success') {
        // Update message cache
        if (data.messages) {
            data.messages.forEach(msg => {
                messageCache.set(msg.id, msg);
            });
        }
        
        // Update chat list if present
        if (data.chats) {
            renderChatList(data.chats);
            
            // Handle chat selection
            if (currentChatTitle) {
                const currentChat = data.chats.find(c => c.title === currentChatTitle);
                if (currentChat) {
                    selectChat(currentChat.title, currentChat.head);
                }
            } else if (data.chats.length > 0) {
                selectChat(data.chats[0].title, data.chats[0].head);
            }
        }

        // Handle message updates
        if (data.type === 'message_update' && data.chat_id === currentChatTitle) {
            data.messages.forEach(msg => {
                messageCache.set(msg.id, msg);
            });
            if (currentChatTitle) {
                const messages = buildMessageChain(data.messages[data.messages.length - 1].id);
                renderMessages(messages);
            }
        }
    }
}

// Modal functions
function showNewChatModal() {
    document.getElementById('newChatModal').classList.add('show');
    document.getElementById('newChatTitle').focus();
}

function closeNewChatModal() {
    document.getElementById('newChatModal').classList.remove('show');
    document.getElementById('newChatTitle').value = '';
}

// Chat creation
async function submitNewChat() {
    const titleInput = document.getElementById('newChatTitle');
    const title = titleInput.value.trim();
    if (!title) return;

    sendWebSocketMessage({
        type: 'new_chat',
        title: title
    });
    
    closeNewChatModal();
}

// Message handling
async function sendMessage() {
    const text = messageInput.value.trim();
    const sendButton = document.querySelector('.send-button');

    if (!text || !currentChatTitle) return;

    try {
        messageInput.disabled = true;
        sendButton.disabled = true;

        sendWebSocketMessage({
            type: 'send_message',
            content: text,
            chat_id: currentChatTitle
        });

        messageInput.value = '';
        messageInput.style.height = '2.5rem';
        messageInput.focus();
    } catch (error) {
        console.error('Error sending message:', error);
        alert('Failed to send message. Please try again.');
    } finally {
        messageInput.disabled = false;
        sendButton.disabled = false;
    }
}

// Build the message chain from head to root
function buildMessageChain(headId) {
    const messages = [];
    let currentId = headId;

    while (currentId) {
        const message = messageCache.get(currentId);
        if (!message) break;
        messages.unshift(message);
        currentId = message.parent;
    }

    return messages;
}

// Chat selection and message rendering
function selectChat(title, headId) {
    currentChatTitle = title;
    currentMessageParentId = headId;

    const messages = buildMessageChain(headId);
    renderMessages(messages);

    // Update UI to show active chat
    document.querySelectorAll('.chat-item').forEach(chat => {
        if (chat.textContent.trim() === title) {
            chat.classList.add('active');
        } else {
            chat.classList.remove('active');
        }
    });

    // Focus message input
    messageInput.focus();
}

function renderChatList(chats) {
    const chatList = document.getElementById('chatList');

    if (chats.length === 0) {
        chatList.innerHTML = `
            <div class="empty-state">
                No chats yet.<br>Create your first chat!
            </div>
        `;
        return;
    }

    chatList.innerHTML = chats.map(chat => `
        <div onclick="selectChat('${escapeHtml(chat.title)}', '${chat.head}')"
             class="chat-item ${chat.title === currentChatTitle ? 'active' : ''}">
            <span>${escapeHtml(chat.title)}</span>
        </div>
    `).join('');
}

function renderMessages(messages) {
    if (messages.length === 0) {
        messageArea.innerHTML = `
            <div class="empty-state">
                No messages yet.<br>Start the conversation!
            </div>
        `;
        return;
    }

    messageArea.innerHTML = `<div class="message-container">${
        messages.map(msg => `
            <div class="message ${msg.role}" data-id="${msg.id}">
                ${formatMessage(msg.content)}
            </div>
        `).join('')
    }</div>`;

    messageArea.scrollTop = messageArea.scrollHeight;
}

// Message formatting
function formatMessage(content) {
    // First escape HTML
    let text = escapeHtml(content);
    
    // Format code blocks
    text = text.replace(/```([^`]+)```/g, (match, code) => `<pre><code>${code}</code></pre>`);
    
    // Format inline code
    text = text.replace(/`([^`]+)`/g, (match, code) => `<code>${code}</code>`);
    
    return text;
}

// Utility functions
function escapeHtml(unsafe) {
    return unsafe
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    connectWebSocket();

    // Setup message input handling
    messageInput.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    });

    // Handle "new chat" modal
    document.getElementById('newChatTitle').addEventListener('keydown', (event) => {
        if (event.key === 'Enter') {
            event.preventDefault();
            submitNewChat();
        }
        if (event.key === 'Escape') {
            closeNewChatModal();
        }
    });
});

// Handle visibility changes
document.addEventListener('visibilitychange', () => {
    if (!document.hidden && (!ws || ws.readyState !== WebSocket.OPEN)) {
        connectWebSocket();
    }
});

// Cleanup on page unload
window.addEventListener('unload', () => {
    if (ws) {
        ws.close();
    }
});
