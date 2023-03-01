use actix_web;
use actix_web::dev::HttpServiceFactory;
use actix_web::web::{post, Data, Path};
use actix_web::{web, App, HttpResponse, HttpServer};
use poker_board::domain::clear_votes::ClearVotes;
use poker_board::event::BoardModifiedEvent;
use poker_board::port::ModifyError;
use poker_board::presentation::input::ClearVotesDto;
use poker_board::presentation::CommandController;
use std::sync::{Arc, Mutex};
use util;
use util::command::Input;
use util::store::EventStore;
use util::use_case;
use util::use_case::{Handler, ResponseHandler};

#[actix_web::post("/clear-votes")]
async fn clear_votes<'a>(
    data: Data<Handler<'a, BoardModifiedEvent, ModifyError>>,
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

struct Query {
    store: Arc<Mutex<poker_board::adapter::Store>>,
}

impl Query {
    fn query_for(&self, id: String) -> HttpResponse {
        let events = self.store.lock().unwrap().get(&id).cloned();
        match events {
            Some(events) => {
                let events = events
                    .iter()
                    .map(|event| event.to_string())
                    .collect::<Vec<String>>();
                HttpResponse::Ok().json(events)
            }
            None => HttpResponse::NotFound().finish(),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let adapter = poker_board::adapter::InMemoryModifyEntityAdapter::default();

    let response_handler = |events: Vec<BoardModifiedEvent>| -> Result<(), String> { Ok(()) };

    let handler = Handler::new(adapter, response_handler);

    let app_data = Data::new(handler);
    // let query = actix_web::web::Data::new(query);
    //
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .service(clear_votes)
            .route("/random", web::get().to(random))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn random() -> HttpResponse {
    HttpResponse::Ok().body("random")
}
