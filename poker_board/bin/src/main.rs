use actix_web;
use actix_web::web::post;
use actix_web::{App, HttpResponse, HttpServer};
use poker_board::domain::clear_votes::ClearVotes;
use poker_board::event::BoardModifiedEvent;
use poker_board::presentation::{ClearVotesDto, Controller};
use std::sync::Arc;

#[actix_web::post("/clear-votes")]
async fn clear_votes(
    data: actix_web::web::Data<Controller<ClearVotesDto>>,
    body: String,
) -> HttpResponse {
    let response = data.handle(body).await;
    response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let adapter = poker_board::adapter::InMemoryModifyEntityAdapter::default();

    let service = poker_board::service::ModifyingService::<ClearVotes, BoardModifiedEvent>::new(
        Box::new(adapter),
    );

    let deserializer =
        Box::new(|req_body: String| serde_json::from_str(&req_body).map_err(|e| e.to_string()));
    let controller = poker_board::presentation::Controller::<ClearVotesDto>::new(
        Box::new(service),
        deserializer,
    );

    let app_data = actix_web::web::Data::new(controller);

    HttpServer::new(move || App::new().app_data(app_data.clone()).service(clear_votes))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
