use actix_web::{middleware::Logger, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bson::doc;
use dotenv;
use env_logger;
use mongodb::{options::ClientOptions, Client};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Clone)]
struct AppState {
    db: mongodb::Database,
}

#[derive(Serialize, Deserialize, Clone)]
struct EncryptedData {
    project_id: String,
    encrypted_content: String,
    access_key: String,
}

#[derive(Deserialize)]
struct StoreData {
    project_id: String,
    encrypted_content: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ShareData {
    project_id: String,
    share_code: String,
    encrypted_content: String,
}

async fn share_config(data: web::Json<ShareData>, state: web::Data<AppState>) -> impl Responder {
    let collection = state.db.collection::<ShareData>("share_data");

    // Check if data already exists
    if let Ok(Some(_)) = collection
        .find_one(doc! { "project_id": &data.project_id }, None)
        .await
    {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Data already exists, use update if you want to modify it"
        }));
    }

    let share_data = ShareData {
        project_id: data.project_id.clone(),
        encrypted_content: data.encrypted_content.clone(),
        share_code: data.share_code.clone(),
    };

    match collection.insert_one(share_data, None).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Data stored successfully.",
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to store data"
        })),
    }
}

// retrieve data using share code as parameter | ShareData
async fn pull_config(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let collection = state.db.collection::<ShareData>("share_data");

    let share_code = req
        .headers()
        .get("X-Share-Code")
        .and_then(|h| h.to_str().ok());
    let share_code = match share_code {
        Some(code) => code,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Missing Share Code"
            }))
        }
    };

    let project_id = req
        .headers()
        .get("X-Project-Id")
        .and_then(|h| h.to_str().ok());
    let project_id = match project_id {
        Some(id) => id,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Missing Project Id"
            }))
        }
    };

    match collection
        .find_one(
            doc! { "share_code": share_code, "project_id": project_id },
            None,
        )
        .await
    {
        Ok(Some(data)) => {
            // Delete the record after retrieval
            collection
                .delete_one(
                    doc! { "share_code": share_code, "project_id": project_id },
                    None,
                )
                .await
                .unwrap();
            HttpResponse::Ok().json(serde_json::json!({
                "encrypted_content": data.encrypted_content
            }))
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "detail": "Data not found"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to retrieve data"
        })),
    }
}

async fn store_data(data: web::Json<StoreData>, state: web::Data<AppState>) -> impl Responder {
    let collection = state.db.collection::<EncryptedData>("encrypted_data");

    // Check if data already exists
    if let Ok(Some(_)) = collection
        .find_one(doc! { "project_id": &data.project_id }, None)
        .await
    {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "detail": "Data already exists, use update if you want to modify it"
        }));
    }

    let access_key = uuid::Uuid::new_v4().to_string();
    let new_data = EncryptedData {
        project_id: data.project_id.clone(),
        encrypted_content: data.encrypted_content.clone(),
        access_key: access_key.clone(),
    };

    match collection.insert_one(new_data, None).await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Data stored successfully",
            "access_key": access_key
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to store data"
        })),
    }
}

async fn retrieve_data(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let collection = state.db.collection::<EncryptedData>("encrypted_data");

    let api_key = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok());
    let api_key = match api_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Missing API Key"
            }))
        }
    };

    match collection
        .find_one(doc! { "access_key": api_key }, None)
        .await
    {
        Ok(Some(data)) => HttpResponse::Ok().json(serde_json::json!({
            "encrypted_content": data.encrypted_content
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "detail": "Data not found"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to retrieve data"
        })),
    }
}

async fn update_data(
    req: HttpRequest,
    data: web::Json<StoreData>,
    state: web::Data<AppState>,
) -> impl Responder {
    let collection = state.db.collection::<EncryptedData>("encrypted_data");

    let api_key = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok());
    let api_key = match api_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Missing API Key"
            }))
        }
    };

    match collection
        .find_one_and_update(
            doc! { "access_key": api_key },
            doc! { "$set": { "encrypted_content": &data.encrypted_content } },
            None,
        )
        .await
    {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Data updated successfully"
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "detail": "Data not found"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to update data"
        })),
    }
}

async fn delete_data(req: HttpRequest, state: web::Data<AppState>) -> impl Responder {
    let collection = state.db.collection::<EncryptedData>("encrypted_data");

    let api_key = req.headers().get("X-API-Key").and_then(|h| h.to_str().ok());
    let api_key = match api_key {
        Some(key) => key,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "detail": "Missing API Key"
            }))
        }
    };

    match collection
        .find_one_and_delete(doc! { "access_key": api_key }, None)
        .await
    {
        Ok(Some(_)) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Data deleted successfully"
        })),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "detail": "Data not found"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "detail": "Failed to delete data"
        })),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let database_name = env::var("DATABASE_NAME").expect("DATABASE_NAME must be set");

    let server_url = env::var("SERVER_URL").expect("SERVER_URL must be set");

    let client_options = ClientOptions::parse(&database_url).await.unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database(&database_name);

    let state = web::Data::new(AppState { db });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .route("/push", web::post().to(store_data))
            .route("/pull", web::get().to(retrieve_data))
            .route("/update", web::put().to(update_data))
            .route("/delete", web::delete().to(delete_data))
            .route("/share", web::post().to(share_config))
            .route("/share", web::get().to(pull_config))
    })
    .bind(server_url)?
    .run()
    .await
}
