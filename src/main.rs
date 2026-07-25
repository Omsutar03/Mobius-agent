use reqwest::Client;
use serde_json::json;
use std::io::{self, Write};
use std::time::Duration;

// Splashscreen
fn splash_screen() -> &'static str {
    r#"
        ███╗   ███╗ ██████╗ ██████╗ ██╗██╗   ██╗███████╗
        ████╗ ████║██╔═══██╗██╔══██╗██║██║   ██║██╔════╝
        ██╔████╔██║██║   ██║██████╔╝██║██║   ██║███████╗
        ██║╚██╔╝██║██║   ██║██╔══██╗██║██║   ██║╚════██║
        ██║ ╚═╝ ██║╚██████╔╝██████╔╝██║╚██████╔╝███████║
        ╚═╝     ╚═╝ ╚═════╝ ╚═════╝ ╚═╝ ╚═════╝ ╚══════╝
        "#
}

// Function to read user prompt
fn get_user_prompt(identifier: &str) -> io::Result<String> {
    print!("{}", identifier);
    io::stdout().flush()?;

    let mut prompt = String::new();
    io::stdin().read_line(&mut prompt)?;

    Ok(prompt.trim().to_string())
}

// Get Mobius's reponse
async fn ask_mobius(
    client: &Client,
    server_url: &str,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let payload = json!({
        "messages": [
            {
                "role": "system",
                "content": "You are Mobius agent, a concise terminal AI assistant. Provide direct, helpful answers."
            },
            {
                "role": "user",
                "content": prompt
            }
        ],
    });

    // Loading animation
    let sprite = tokio::spawn(async move {
        let frames: [&str; 12] = [
            "(•‿•)ゝ",
            "(•‿•)ゞ",
            "(•‿•)ゝ",
            "(•_•)ゞ",
            "(•_•)ゝ",
            "(•_•)ゞ",
            "(-_-)ゝ",
            "(-_-)ゞ",
            "(-_-)ゝ",
            "(⇀‸↼)ゞ",
            "(⇀‸↼)ゝ",
            "(⇀‸↼)ゞ",
        ];

        let mut f = 0;

        loop {
            print!("\r\x1B[2KMobius is thinking... {}", frames[f % 12]); // 12 is hardcoded size of frames (for improved performance)
            let _ = io::stdout().flush();
            tokio::time::sleep(Duration::from_millis(300)).await;
            f += 1;
        }
    });

    // Send the HTTP request
    let response_result = client.post(server_url).json(&payload).send().await;

    // Stop sprite animation as soon as response is received and clear the line
    sprite.abort();
    print!("\r\x1B[2K");
    let _ = io::stdout().flush();

    // Now safely evaluate the network result
    let response = response_result?;

    // Check HTTP status code explicitly
    if !response.status().is_success() {
        return Err(format!("Server returned HTTP status {}", response.status()).into());
    }

    // Parse JSON body safely
    let res_json: serde_json::Value = response.json().await?;

    // Extract string content and return Result to main
    if let Some(text) = res_json["choices"][0]["message"]["content"].as_str() {
        Ok(text.trim().to_string())
    } else {
        Err("Could not find valid text in the model response".into())
    }
}

// Main function as async
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", splash_screen());

    let client = Client::new();
    let server_url = "http://localhost:8080/v1/chat/completions";

    // Main agent loop
    loop {
        // Get user prompt
        let prompt = get_user_prompt("\nYou: ")?;

        // Check prompt for special cases
        if prompt.is_empty() {
            continue;
        }

        if prompt.eq_ignore_ascii_case("/q") || prompt.eq_ignore_ascii_case("/quit") {
            println!("Shutting down Mobius....");
            break;
        }

        // Return Mobius's reponse
        match ask_mobius(&client, server_url, &prompt).await {
            Ok(response) => println!("\nMobius: {}", response),
            Err(err) => println!("\nError: {}", err),
        }
    }

    Ok(())
}
