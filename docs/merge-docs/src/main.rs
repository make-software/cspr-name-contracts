// Read `sequence-diagrams` directory, updates the plum code and returns all as
// one file: `casper-name-sequence-diagrams.txt`.

fn read_doc_file(file_name: &str) -> String {
    let path = format!("../sequence-diagrams/{}", file_name);
    let mut result = String::new();
    result.push_str(&format!("FILE: {}\n\n", file_name));
    let content = &std::fs::read_to_string(path).unwrap();
    let content = embed_pulm(content);
    result.push_str(&content);
    result
}

// For every expression in the content that looks like:
// 
// [🔗](puml/set-transfer-filter.puml)
//
// Read the content of the puml file and embed it in the content.
fn embed_pulm(content: &str) -> String {
    let mut result = String::new();
    // read the file line by line
    for line in content.lines() {
        if line.starts_with("![](puml") {
            continue;
        }
        if line.starts_with("[🔗](") {
            let start = line.find("[🔗](").unwrap();
            let end = line.find(")").unwrap();
            let file_name = &line[start + 7..end];
            let path = format!("../sequence-diagrams/{}", file_name);
            let puml_content = &std::fs::read_to_string(path).unwrap();
            result.push_str("\n```puml\n");
            result.push_str(&puml_content);
            result.push_str("```\n");
        } else {
            result.push_str(line);
        }
        result.push_str("\n");
    }

    result    
}

fn main() {
    let mut content = String::new();
    content.push_str(&read_doc_file("Admin functions.md"));
    content.push_str(&read_doc_file("Buy a cspr name.md"));
    content.push_str(&read_doc_file("Renew a cspr name.md"));
    content.push_str(&read_doc_file("Resolution.md"));
    content.push_str(&read_doc_file("Secondary-sale market.md"));
    
    // Write the content to a file
    std::fs::write("casper-name-sequence-diagrams.txt", content).unwrap();
}
