use serde::Serialize;

#[derive(Serialize)]
pub struct CliResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub count: usize,
}

pub fn output_success<T: Serialize>(data: T) {
    let response = CliResponse {
        success: true,
        data: Some(data),
        error: None,
    };
    println!("{}", serde_json::to_string(&response).unwrap());
}

pub fn output_list<T: Serialize>(items: Vec<T>) {
    let count = items.len();
    let list = ListResponse { items, count };
    output_success(list);
}

pub fn output_error(message: &str) -> ! {
    let response: CliResponse<()> = CliResponse {
        success: false,
        data: None,
        error: Some(message.to_string()),
    };
    eprintln!("{}", serde_json::to_string(&response).unwrap());
    std::process::exit(1);
}
