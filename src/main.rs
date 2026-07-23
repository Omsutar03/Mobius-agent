use std::io::{self, Write};
use std::time::Duration;
use reqwest::Client;
use serde_json::json;

// The tokio macro wraps main function so it can run async code
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Mobius Initialized. Type '/bye', '/q' or '/quit' to close.");
    
    // HTTP client which'll be reused for every request
    let client = Client::new();
    let server_url = "http://localhost:8080/v1/chat/completions";

    // The main Agent Loop
    loop {
        // 1. Prompt the user
        print!("\nYou: ");
        io::stdout().flush()?; // Ensure the above message prints before waiting for user input

        // 2. Read the user's input
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit commands
        if input.eq_ignore_ascii_case("/bye") || input.eq_ignore_ascii_case("/q") || input.eq_ignore_ascii_case("/quit") {
            println!("Shutting down Mobius...");
            break;
        }
        
        if input.is_empty() {
            continue;
        }

        // 3. Construct the JSON payload for llama.cpp
        // Using the serde_json::json! macro to make it look just like standard JSON
        let payload = json!({
            "messages": [
                {
                    "role": "system",
                    "content": "You are Mobius, a concise terminal AI assistant running locally on Arch Linux. Provide direct, helpful answers."
                },
                {
                    "role": "user",
                    "content": input
                }

            ],
            
            //"stream": true,
        });
        
        // 4. Start the spinner in a background task
        let spinner = tokio::spawn(async move {
            // A braille spinner animation array
            // let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let frames = [
              "(-_-)ゝ", 
              "(-_-)ゞ", 
              "(•_•)ヽ", 
              "(•_•)ヾ", 
              "(•_•)ゝ", 
              "(•_•)ゞ", 
              "(._.)ヽ", 
              "(._.)ヾ", 
              "(°_°)ゝ", 
              "(-_-)ゞ"  
            ];

            let mut i = 0;
            
            loop {
                // \r moves the cursor to the start of the line.
                // \x1B[2K is the ANSI escape code to clear the entire line [1.2.1].
                print!("\r\x1B[2KMobius is thinking... {}", frames[i % frames.len()]);
                
                // Flush forces the terminal to draw immediately
                let _ = io::stdout().flush();
                
                // Pause for 300 milliseconds before drawing the next frame
                tokio::time::sleep(Duration::from_millis(300)).await;
                i += 1;
            }
        });

        // 5. Send the request
        // Because the spinner is running in a spawned background task, 
        // the program will wait right here for the server to reply.
        let response = client.post(server_url)
            .json(&payload)
            .send()
            .await?;
            
        // 6. Stop the spinner and clean up the terminal line
        spinner.abort(); // Kills the background task instantly
        print!("\r\x1B[2K"); // Erase the spinner text completely
        let _ = io::stdout().flush();

        // 7. Parse the response and print it
        if response.status().is_success() {
            // Convert the response body to a JSON object
            let res_json: serde_json::Value = response.json().await?;
            
            // Extract the text content from the deep JSON structure
            // Format: { "choices": [ { "message": { "content": "..." } } ] }
            if let Some(text) = res_json["choices"][0]["message"]["content"].as_str() {
                println!("\nMobius: {}", text.trim());
            } else {
                println!("\n[Error: Could not parse text from response]");
            }
        } else {
            println!("\n[Server Error: {}]", response.status());
        }
    }

    Ok(())
}