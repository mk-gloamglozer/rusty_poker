use actix_web;
use actix_web::web::{Data, Path};
use actix_web::{App, HttpResponse, HttpServer};
use poker_board::command::adapter::{basic_transaction_store, in_memory_store};
use poker_board::command::domain::clear_votes::ClearVotes;
use poker_board::command::domain::Board;
use poker_board::command::event::BoardModifiedEvent;
use std::fmt::Debug;
use util;
use util::presentation::Input;
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

#[actix_web::post("/clear-votes")]
async fn clear_votes(
    data: Data<UseCase<BoardModifiedEvent, Board, String, String>>,
    body: String,
) -> HttpResponse {
    let body = match serde_json::from_str::<Input<ClearVotes>>(&body) {
        Ok(body) => body,
        Err(err) => {
            log::error!("Error parsing body: {}", err);
            return HttpResponse::BadRequest().finish();
        }
    };

    let response = data.execute(body.command(), body.id()).await;
    response
        .log()
        .map(|_| HttpResponse::Ok().finish())
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

trait Qry {
    fn query_for(&self, id: String) -> HttpResponse;
}

#[actix_web::get("/events/{id}")]
async fn get_events(data: Data<dyn Qry>, path: Path<String>) -> HttpResponse {
    data.query_for(path.into_inner())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let store = in_memory_store();
    let transactor = basic_transaction_store(store);

    let use_case = UseCase::new(transactor);
    let app_data = Data::new(use_case);
    HttpServer::new(move || App::new().app_data(app_data.clone()).service(clear_votes))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
