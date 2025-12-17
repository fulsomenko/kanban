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

/// Outputs an error response to stderr and returns an error for proper propagation.
///
/// Returns an `anyhow::Error` to allow callers to handle the error appropriately
/// and enable proper cleanup. The CLI's main function handles the exit code.
pub fn output_error(message: &str) -> anyhow::Result<()> {
    let response: CliResponse<()> = CliResponse {
        success: false,
        api_version: env!("CARGO_PKG_VERSION"),
        data: None,
        error: Some(message.to_string()),
    };
    eprintln!("{}", serde_json::to_string(&response).unwrap());
    anyhow::bail!("{}", message)
}
