use actix_web;
use actix_web::web::{Data, Path};
use actix_web::{App, HttpResponse, HttpServer};
use poker_board::command::adapter::{basic_transaction_store, in_memory_store};
use poker_board::command::event::BoardModifiedEvent;
use poker_board::command::{Board, BoardCommand};
use std::fmt::Debug;
use util;
use util::query::Query;
use util::use_case::UseCase;

trait Log {
    fn log(self) -> Self;
}

impl<T> Log for T
where
    T: Debug,
{
    fn log(self) -> Self {
        log::info!("Log: {:?}", self);
        self
    }
}

#[actix_web::post("/board/{id}")]
async fn clear_votes(
    data: Data<UseCase<BoardModifiedEvent, Board, String, String>>,
    body: String,
    path: Path<String>,
) -> HttpResponse {
    let command = match serde_json::from_str::<BoardCommand>(&body) {
        Ok(body) => body,
        Err(err) => {
            log::error!("Error parsing body: {}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    let key = path.into_inner();
    let response = data.execute(&command, &key).await;
    response
        .log()
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[actix_web::get("/board/{id}")]
async fn get_events(
    query: Data<Query<poker_board::query::Board>>,
    path: Path<String>,
) -> HttpResponse {
    let key = path.into_inner();
    log::debug!("Getting board with key: {}", key);
    let response = query.get(&key).await;
    response
        .log()
        .map(|board| HttpResponse::Ok().json(board))
        .unwrap_or_else(|_| HttpResponse::NotFound().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let store = in_memory_store::<Board>();
    let transactor = basic_transaction_store(store.clone());

    let use_case = UseCase::new(transactor);

    let query = Query::new(store.loader_for::<poker_board::query::Board>());

    let use_case_data = Data::new(use_case);
    let query_data = Data::new(query);

    HttpServer::new(move || {
        App::new()
            .app_data(query_data.clone())
            .app_data(use_case_data.clone())
            .service(clear_votes)
            .service(get_events)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
