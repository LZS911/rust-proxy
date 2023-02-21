use reqwest::{
    header::{HeaderMap, HeaderValue, USER_AGENT},
    Error, Response,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

mod proxy;

use crate::proxy::proxy::Proxy;

#[tokio::main]
async fn main() {
    match get_swagger_json().await {
        Err(e) => println!("error is {}", e),
        Ok(res) => {
            if res.status().is_success() {
                let json = res.text().await.ok().unwrap();
                let mut api = ApiJson {
                    api_list: vec![],
                    definitions: vec![],
                    json,
                };

                api.start();
            } else {
                let err_message = res.text().await.ok().unwrap();
                println!("error is {}", err_message);
            }
        }
    }
}

async fn get_swagger_json() -> Result<Response, Error> {
    // println!("请输入后端 swagger json 文件的请求地址: ");
    // let mut swagger_json_url = String::new();
    // let mut token = String::new();
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();

    // io::stdin()
    //     .read_line(&mut swagger_json_url)
    //     .expect("Failed to read line");

    // println!("请输入项目 Token: ");

    // io::stdin()
    //     .read_line(&mut token)
    //     .expect("Failed to read line");

    headers.insert(USER_AGENT, HeaderValue::from_static("rust client"));
    // headers.insert(
    //     AUTHORIZATION,
    //     HeaderValue::from_str(token.as_str().trim()).ok().unwrap(),
    // );
    Ok(client
        .get(
            "https://raw.githubusercontent.com/actiontech/sqle/main/sqle/docs/swagger.json
    ",
        )
        .headers(headers)
        .send()
        .await?)
}

#[derive(Deserialize, Debug)]
struct SuccessResponses {
    schema: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
struct Infos {
    tags: Vec<String>,
    operationId: String,
    responses: HashMap<String, Value>,
}

#[derive(Deserialize, Debug, Clone, Copy)]
enum ApiMethods {
    Get,
    Post,
    Patch,
    Delete,
}

#[derive(Deserialize, Debug)]
struct ApiObject {
    tags: Vec<String>,
    operation_id: String,
    url: String,
    method: ApiMethods,
    definitions: String,
}

#[derive(Deserialize, Debug)]
struct DefinitionObject {
    key: String,
    properties: HashMap<String, Value>,
}
#[derive(Deserialize, Debug)]
struct ApiJson {
    json: String,
    api_list: Vec<ApiObject>,
    definitions: Vec<DefinitionObject>,
}

impl ApiJson {
    pub fn get_api_infos(&mut self, value: Value) {
        let paths: HashMap<String, Value> =
            serde_json::from_str(value.to_string().as_str()).unwrap();
        for (url, value) in paths {
            if url == "/v1/oauth2/link" {
                continue;
            }
            let url_info: HashMap<String, Value> =
                serde_json::from_str(value.to_string().as_str()).unwrap();
            for (method, value) in url_info {
                let api_method = get_methods(&method);
                let obj: Infos = serde_json::from_str(value.to_string().as_str()).unwrap();

                let responses: HashMap<String, Value> = obj.responses;

                let schema: SuccessResponses =
                    serde_json::from_str(responses.values().nth(0).unwrap().to_string().as_str())
                        .unwrap();

                let api_infos = ApiObject {
                    tags: obj.tags,
                    operation_id: obj.operationId,
                    url: url.clone(),
                    method: api_method,
                    definitions: schema.schema.values().nth(0).unwrap().to_string(),
                };

                let _ = &self.api_list.push(api_infos);
            }
        }
    }

    pub fn get_definitions(&mut self, value: Value) {
        let definitions: HashMap<String, Value> =
            serde_json::from_str(value.to_string().as_str()).unwrap();

        for (key, value) in definitions {
            let properties: HashMap<String, Value> =
                serde_json::from_str(value.to_string().as_str()).unwrap();
            let obj = DefinitionObject { key, properties };
            let _ = &self.definitions.push(obj);
        }
    }

    pub fn start(&mut self) {
        let deserialized: HashMap<String, Value> = serde_json::from_str(&self.json).unwrap();

        for (key, value) in deserialized {
            if key == "paths" {
                self.get_api_infos(value)
            } else if key == "definitions" {
                self.get_definitions(value)
            }
        }

        let mut proxy = Proxy::new("127.0.0.1:23333").unwrap();
        proxy.run();
    }
}

fn get_methods(method: &String) -> ApiMethods {
    if method == "get" {
        return ApiMethods::Get;
    } else if method == "post" {
        return ApiMethods::Post;
    } else if method == "patch" {
        return ApiMethods::Patch;
    }

    ApiMethods::Delete
}
