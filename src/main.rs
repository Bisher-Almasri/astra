use astra::{browser::Browser, js};

fn main() {
    println!("Astra Browser Engine v0.1.0");

    let mut browser = Browser::new();

    match browser.load("test/index.html") {
        Ok(pixels) => {
            println!(
                "Loaded '{}' — pixel buffer: {} bytes ({} x {} RGBA)",
                browser.current_url().unwrap_or(""),
                pixels.len(),
                browser.viewport_width as u32,
                browser.viewport_height as u32,
            );
        }
        Err(e) => {
            eprintln!("Failed to load page: {}", e);
            return;
        }
    }

    let _ = browser.load("test/index.html");
    let html = browser.get_html("test/index.html").unwrap_or("").to_string();
    println!(
        "History length: {}  |  current: {}",
        browser.history.len(),
        browser.current_url().unwrap_or("(none)")
    );

    if browser.can_go_back() {
        let prev = browser.navigate_back().unwrap();
        println!("Navigated back to: {}", prev);
    }

    println!("can_go_back={} can_go_forward={}", browser.can_go_back(), browser.can_go_forward());

    if browser.can_go_forward() {
        let next = browser.navigate_forward().unwrap();
        println!("Navigated forward to: {}", next);
    }    

    let scripts = js::extract_scripts(&html);
    let mut runtime = js::JsRuntime::new();
    for script in scripts {
        match runtime.execute(&script) {
            Ok(_) => println!("Executed script successfully."),
            Err(e) => eprintln!("JS Error: {}", e.message),
        }
    }
    println!("Done.");
}
