use serde_json::Value;

pub fn list_projects(config: &Value) {
    println!("Projects in local config:");
    for (project, _) in config.as_object().unwrap() {
        if project != "api_url" {
            println!("- {}", project);
        }
    }
}
