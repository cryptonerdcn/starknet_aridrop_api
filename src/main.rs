use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize)]
struct EligibleResponse {
    identity: String,
    amount: String,
    merkle_index: String,
    contract_address: String,
    #[serde(rename = "type")]
    contract_type: String,
    merkle_path: Vec<String>,
    merkle_path_len: usize,
}

async fn get_eligible_info(path: web::Path<String>) -> impl Responder {
    let identity = path.into_inner(); 
    let conn = match Connection::open("contracts.db") {
        Ok(conn) => conn,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to connect to database"),
    };

    let mut stmt = match conn.prepare(
        "SELECT e.identity, e.amount, e.merkle_index, c.contract_address, c.contract_type
         FROM eligibles e
         JOIN contracts c ON e.contract_id = c.id
         WHERE e.identity = ?1",
    ) {
        Ok(stmt) => stmt,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to prepare query"),
    };

    let eligible_info = stmt.query_map(params![identity], |row| {
        Ok(EligibleResponse {
            identity: row.get(0)?,
            amount: row.get(1)?,
            merkle_index: row.get(2)?,
            contract_address: row.get(3)?,
            contract_type: row.get(4)?,
            merkle_path: vec![], // This will be populated later
            merkle_path_len: 0,  // This will be set later
        })
    }).unwrap().flatten().next();

    match eligible_info {
        Some(mut info) => {
            // Query for merkle_path based on the identity
            let mut stmt = conn.prepare(
                "SELECT path FROM merkle_paths WHERE eligible_id = 
                (SELECT id FROM eligibles WHERE identity = ?1)"
            ).unwrap();

            let merkle_paths = stmt.query_map(params![identity], |row| {
                Ok(row.get::<_, String>(0)?)
            }).unwrap().flatten().collect::<Vec<String>>();

            info.merkle_path = merkle_paths;
            info.merkle_path_len = info.merkle_path.len();

            HttpResponse::Ok().json(info)
        },
        None => HttpResponse::NotFound().body("Identity not found"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route(
            "/eligible/{identity}", web::get().to(get_eligible_info)
        )
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
