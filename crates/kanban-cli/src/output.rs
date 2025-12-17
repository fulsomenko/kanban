use serde::Serialize;

#[derive(Serialize)]
pub struct CliResponse<T: Serialize> {
    pub success: bool,
    pub api_version: &'static str,
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
        api_version: env!("CARGO_PKG_VERSION"),
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

/// Outputs an error response to stderr and terminates the process.
///
/// This function uses the never type (`!`) because it always exits the process
/// with code 1 after printing the error. This is intentional CLI behavior to
/// signal failure to shell scripts and CI pipelines.
pub fn output_error(message: &str) -> ! {
    let response: CliResponse<()> = CliResponse {
        success: false,
        api_version: env!("CARGO_PKG_VERSION"),
        data: None,
        error: Some(message.to_string()),
    };
    eprintln!("{}", serde_json::to_string(&response).unwrap());
    std::process::exit(1);
}
