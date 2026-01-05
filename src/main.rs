use std::fs;

mod css;
mod html;

fn main() {
    match fs::read_to_string("test/index.html") {
        Ok(contents) => {
            let mut parser = html::HtmlParser::new(contents);

            match parser.parse() {
                Ok(_) => println!("HTML parsed successfully"),
                Err(e) => println!("Parse error: {}", e),
            }
        }
        Err(e) => {
            println!("Failed to read file: {}", e);
        }
    }

    println!("Astra Browser Engine v0.1.0");
}
