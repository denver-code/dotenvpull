use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use bson::doc;
use dotenv::dotenv;
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
    dotenv().ok();

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
            .route("/store", web::post().to(store_data))
            .route("/retrieve", web::get().to(retrieve_data))
            .route("/update", web::put().to(update_data))
            .route("/delete", web::delete().to(delete_data))
    })
    .bind(server_url)?
    .run()
    .await
}
