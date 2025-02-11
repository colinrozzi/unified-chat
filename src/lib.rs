// [Previous imports remain the same...]

impl State {
    // [Previous method implementations remain the same...]

    // Create necessary directories
    fn ensure_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Create main chat directory
        let base_path = format!("{}/data/{}", self.base_directory, self.chat_directory);
        log(&format!("Creating directory at: {}", base_path));
        create_dir(&base_path)?;
        
        // Initialize chats.txt if it doesn't exist
        let chats_path = format!("{}/chats.txt", base_path);
        if !path_exists(&chats_path).expect("Failed to check path existence") {
            log(&format!("Initializing chats.txt at: {}", chats_path));
            write_file(&chats_path, &serde_json::to_string(&Vec::<String>::new())?).unwrap();
        }
        
        Ok(())
    }
}

impl ActorGuest for Component {
    fn init() -> Vec<u8> {
        log("Initializing unified chat actor");

        // Get the current directory
        let base_directory = match bindings::ntwk::theater::filesystem::current_dir() {
            Ok(dir) => dir,
            Err(e) => {
                log(&format!("Error getting current directory: {}", e));
                "/".to_string()
            }
        };
        log(&format!("Base directory: {}", base_directory));

        // Read API key
        let api_key = read_file("api-key.txt").unwrap();
        let api_key = String::from_utf8(api_key).unwrap().trim().to_string();

        let initial_state = State {
            chat_directory: "chats".to_string(),
            base_directory,
            api_key,
            connected_clients: HashMap::new(),
        };

        // Ensure directories exist
        if let Err(e) = initial_state.ensure_directories() {
            log(&format!("Error ensuring directories exist: {}", e));
        }

        serde_json::to_vec(&initial_state).unwrap()
    }
}

// [Rest of the file remains the same...]