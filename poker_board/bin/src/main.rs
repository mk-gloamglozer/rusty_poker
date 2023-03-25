use actix_web::web::{Data, Path};
use actix_web::{web, App, HttpResponse, HttpServer};
use poker_board::command::adapter::{CombinedEventStore, DefaultStore, NoRetry};
use poker_board::command::event::{
    BoardModifiedEvent, CombinedEvent, VoteTypeEvent, VoteValidation,
};
use poker_board::command::BoardCommand;
use std::fmt::Debug;
use util::query::Query;
use util::use_case::UseCase;

use actix_web_actors::ws;
use poker_board::query;
use websockets::store::StoreInterface;
use websockets::{store, websocket};

async fn board_ws(
    r: actix_web::HttpRequest,
    stream: web::Payload,
    path: Path<String>,
    update_store: Data<StoreInterface>,
    use_case: Data<UseCase<CombinedEvent>>,
) -> actix_web::Result<HttpResponse> {
    let board_id = path.into_inner();
    ws::start(
        websocket::WebSocket::new(
            board_id,
            update_store.into_inner(),
            use_case.into_inner(),
            "test".to_string(),
        ),
        &r,
        stream,
    )
    .log()
}

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
async fn modify_board(
    data: Data<UseCase<CombinedEvent>>,
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
    let response = data.execute(&key, &command).await;
    response
        .log()
        .map(|events| {
            serde_json::to_string(&events)
                .map(|body| HttpResponse::Ok().body(body))
                .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
        })
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}

#[actix_web::get("/board/{id}")]
async fn get_events(query: Data<Query<BoardModifiedEvent>>, path: Path<String>) -> HttpResponse {
    let key = path.into_inner();
    log::debug!("Getting board with key: {}", key);
    let response = query
        .query::<query::presentation::BoardPresentation>(&key)
        .await;
    response
        .log()
        .map(|board| HttpResponse::Ok().json(board))
        .unwrap_or_else(|_| HttpResponse::NotFound().finish())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let vote_type_store = DefaultStore::<VoteTypeEvent>::new(vec![VoteTypeEvent::VoteTypeAdded {
        vote_validation: VoteValidation::AnyNumber,
        vote_type_id: "1".to_string(),
    }]);

    let store = store::create_store();
    let combined_write_store =
        CombinedEventStore::new(store.clone(), vote_type_store.clone(), store.clone());
    let combined_read_store =
        CombinedEventStore::new(store.clone(), vote_type_store, store.clone());

    let transaction = util::transaction::Transaction::<Vec<CombinedEvent>>::new(
        NoRetry::new(),
        combined_write_store,
        combined_read_store,
    );

    let use_case = UseCase::new(transaction);
    let query = Query::<BoardModifiedEvent>::new(store.clone());

    let use_case_data = Data::new(use_case);
    let query_data = Data::new(query);

    HttpServer::new(move || {
        App::new()
            .route("/ws/board/{id}", web::get().to(board_ws))
            .app_data(Data::new(store.clone()))
            .app_data(query_data.clone())
            .app_data(use_case_data.clone())
            .service(modify_board)
            .service(get_events)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
