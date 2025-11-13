mod server;

use crate::server::ApiDoc;
use std::fs;
use utoipa::OpenApi;

fn main() {
    let doc = gen_my_openapi();
    fs::write("./OpenAPI.json", doc).expect("OpenAPI write error");
}

fn gen_my_openapi() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap()
}
